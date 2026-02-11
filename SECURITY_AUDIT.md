# Security Audit Report — OTD (One-Time Download)

**Date:** 2026-02-11  
**Auditor:** Automated Security Review  
**Codebase:** otd v0.1.0 (7 source files + 1 HTML template)  
**Scope:** Full source review of all Rust source files and static HTML

---

## Executive Summary

OTD is a dual-port file sharing server with an admin interface and a download server. The audit identified **3 critical**, **4 high**, **4 medium**, and **4 low/informational** findings. The most severe issues are the unauthenticated admin interface exposed on all network interfaces, a race condition in the one-time download mechanism, and multiple XSS vulnerabilities in the web UI.

---

## Findings

### 1. Unauthenticated Admin Interface on 0.0.0.0 — CRITICAL

**Location:** `src/config.rs:66-67`, `src/server.rs` (admin server), `src/handlers.rs` (all admin endpoints)

**Description:** The admin server binds to `0.0.0.0` by default with **zero authentication**. Any host on the network can:
- Browse the entire `base_path` directory tree via `/api/browse`
- Generate download tokens for any file via `POST /api/generate`
- List all active tokens (including full filesystem paths) via `/api/tokens`
- Toggle one-time download enforcement via `/config/one-time/false`

**Impact:** Complete unauthorized access to all served files. An attacker on the same network can exfiltrate any file under `base_path`.

**Recommendation:**
- Default `admin_host` to `127.0.0.1` instead of `0.0.0.0`
- Implement authentication (at minimum HTTP Basic Auth or a shared secret)
- Add a firewall/bind warning in documentation

---

### 2. One-Time Download Race Condition (TOCTOU) — CRITICAL

**Location:** `src/handlers.rs:248-253` (the `download()` method)

```rust
if item.downloaded.load(Ordering::Relaxed) {
    return Ok(HttpResponse::gone());
}
// Mark as downloaded
item.downloaded.store(true, Ordering::Relaxed);
```

**Description:** The check-then-set on the `AtomicBool` uses `Ordering::Relaxed` and is **not atomic as a combined operation**. Two concurrent requests can both read `false`, then both set `true`, resulting in **two successful downloads** of a "one-time" link.

Even with stronger ordering, a separate load + store is fundamentally a TOCTOU race. The correct fix requires `compare_exchange` (CAS):

```rust
if item.downloaded.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
    return Ok(HttpResponse::gone());
}
```

**Impact:** The core security guarantee of the application (one-time downloads) is broken under concurrent access.

**Recommendation:** Replace the load/store pair with `compare_exchange` or `swap`.

---

### 3. Stored XSS via Filenames in Web UI — CRITICAL

**Location:** `static/index.html` — `renderFiles()`, `renderStagedFiles()`, `renderTokens()`, `updateBreadcrumb()`

**Description:** The JavaScript constructs HTML via string concatenation and assigns it with `innerHTML`. Filenames, paths, and token names are interpolated **without any HTML escaping**.

A malicious filename like `<img src=x onerror=alert(document.cookie)>` would execute arbitrary JavaScript in any browser viewing the admin interface.

Specific vulnerable patterns:
```javascript
// renderFiles() — file.name and file.path injected raw into HTML
html += `<span class="file-name">${file.name}</span>`;

// updateBreadcrumb() — path parts injected into onclick and text
breadcrumbHtml += `...onclick="loadFiles('${currentPath}')"...>${part}</span>`;

// renderTokens() — token.name and token.download_url injected raw
html += `<span class="token-name">${token.name}</span>`;
html += `<div class="token-url">${token.download_url || 'N/A'}</div>`;
html += `...onclick="copyToClipboard('${token.download_url || ''}')"...`;
```

**Impact:** Any user who can create a file with a malicious name in the served directory can execute arbitrary JavaScript in the admin interface. This could be used to exfiltrate tokens, generate new download links, or pivot further.

**Recommendation:**
- Use `textContent` instead of `innerHTML` for user-controlled data
- Or implement an HTML escaping function and apply it to all interpolated values
- Use DOM APIs (`createElement`, `setAttribute`) instead of string concatenation

---

### 4. Path Traversal via Symlinks — HIGH

**Location:** `src/handlers.rs:186-189` (browse), `src/handlers.rs:221-224` (generate_link)

```rust
let full_path = self.state.base_path.join(path);
if !full_path.starts_with(&self.state.base_path) {
    return Ok(HttpResponse::forbidden());
}
```

**Description:** The `starts_with()` check operates on the **logical path**, not the **canonicalized path**. If a symlink inside `base_path` points to a directory outside it, the check passes because the logical path still starts with `base_path`.

Example: If `base_path/link` → `/etc/`, then `base_path/link/passwd` passes `starts_with(base_path)`.

Additionally, `base_path` itself is never canonicalized (see `Config::load()` and `Server::new()`), so if `base_path` contains symlinks, the comparison may behave unexpectedly.

**Impact:** Read access to arbitrary files on the system via symlinks within the base directory.

**Recommendation:**
- Canonicalize both `base_path` (at startup) and `full_path` (at request time) using `std::fs::canonicalize()`
- Check `starts_with()` on the canonicalized paths
- Consider rejecting symlinks entirely with a configuration option

---

### 5. URL Decoding Produces Invalid UTF-8 Handling — HIGH

**Location:** `src/handlers.rs:398-419` (url_decode)

```rust
if let Ok(byte) = u8::from_str_radix(&hex, 16) {
    result.push(byte as char);
}
```

**Description:** The `url_decode` function decodes `%XX` sequences byte-by-byte and casts each byte directly to a `char`. This is incorrect for multi-byte UTF-8 sequences (e.g., `%C3%A9` for `é` would produce two separate Latin-1 characters instead of one UTF-8 character).

More critically for security, `%2F` decodes to `/`, which means an attacker can send `path=..%2F..%2Fetc%2Fpasswd`. After URL decoding, this becomes `../../etc/passwd`. The `base_path.join("../../etc/passwd")` creates a path that *may* escape the base directory.

However, `PathBuf::join` with an argument starting with `..` produces `base_path/../../etc/passwd`, and `starts_with(base_path)` on the **non-canonicalized** path will actually catch `..` components in most cases because the string won't literally start with the base path after the `..` traverses up. **But combined with finding #4 (no canonicalization), edge cases exist** — e.g., if `base_path` is `/a/b` and the decoded path is `../b/../../etc/passwd`, the joined path is `/a/b/../b/../../etc/passwd` which starts with `/a/b` lexically but resolves to `/etc/passwd`.

**Impact:** Potential path traversal depending on base_path structure. Incorrect character handling for non-ASCII filenames.

**Recommendation:**
- Canonicalize paths after joining (see finding #4)
- Use a proper URL decoding library that handles UTF-8 multi-byte sequences correctly
- Reject paths containing `..` components entirely as an additional defense

---

### 6. Request Truncation / Body Loss — HIGH

**Location:** `src/server.rs:118-120` (admin server), `src/server.rs:157-159` (download server)

```rust
let mut buffer = vec![0; handler.config.buffer_size];
match stream.read(&mut buffer).await {
    Ok(n) if n > 0 => {
        let request_str = String::from_utf8_lossy(&buffer[..n]);
```

**Description:** The server performs a **single `read()` call** with a fixed buffer (default 8KB). Problems:

1. **POST body truncation:** If headers + body exceed 8KB, the body is silently truncated. The `extract_body()` method doesn't check `Content-Length` — it just takes whatever is after `\r\n\r\n` in the buffer. For `POST /api/generate` with many paths, this could cause JSON parse errors or, worse, partial parsing.

2. **No Content-Length validation:** The server never checks if it received the full body as indicated by `Content-Length`.

3. **Large request handling:** `max_request_size` (10MB) is defined in config but **never used anywhere in the code**. There's no enforcement.

4. **`from_utf8_lossy`:** Invalid UTF-8 bytes are replaced with `�`, potentially altering the request content silently.

**Impact:** POST requests with large bodies will fail silently or produce unexpected behavior. No protection against oversized requests.

**Recommendation:**
- Read in a loop until `Content-Length` bytes are received or a maximum is reached
- Enforce `max_request_size`
- Return 413 Payload Too Large for oversized requests
- Use proper UTF-8 validation instead of lossy conversion

---

### 7. Denial of Service: Unbounded Token Generation & ZIP Bombs — HIGH

**Location:** `src/handlers.rs:207-247` (generate_link), `src/handlers.rs:289-340` (serve_as_zip)

**Description:** Multiple DoS vectors exist:

1. **Unlimited token generation:** No rate limiting on `POST /api/generate`. An attacker can generate millions of tokens, consuming unbounded memory (tokens are stored in a `HashMap` that never shrinks).

2. **ZIP of huge directories:** Requesting a ZIP of a large directory tree causes the server to:
   - Recursively walk the entire tree (`WalkDir`)
   - Read every file entirely into memory (`std::fs::read`)
   - Build the entire ZIP in memory before sending

   A directory with many large files will cause OOM.

3. **Large single file serving:** `serve_single_file` reads the entire file into memory with `std::fs::read`. A multi-GB file will cause OOM.

4. **No connection limits:** The server accepts unlimited concurrent connections with no backpressure.

**Impact:** Trivial denial of service by any network user (admin interface is unauthenticated).

**Recommendation:**
- Add rate limiting on token generation
- Implement streaming file/ZIP responses instead of buffering in memory
- Add maximum file size limits
- Limit concurrent connections
- Add token expiration/cleanup

---

### 8. Information Leakage via API Endpoints — MEDIUM

**Location:** `src/handlers.rs:270-284` (list_tokens)

```rust
"paths": item.paths.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>()
```

**Description:** The `/api/tokens` endpoint exposes **full absolute filesystem paths** for all download items. Combined with the unauthenticated admin interface, this leaks the server's directory structure to any network user.

The `/api/browse` endpoint also reveals the directory structure under `base_path`, which may contain sensitive organizational information.

**Impact:** Information disclosure of filesystem layout, useful for further attacks.

**Recommendation:**
- Remove or redact full paths from the `/api/tokens` response
- Only show relative paths
- Require authentication (see finding #1)

---

### 9. Missing Security Headers — MEDIUM

**Location:** `src/http.rs` (HttpResponse builder — no security headers set anywhere)

**Description:** No security headers are set on any response:
- No `Content-Security-Policy` — allows inline scripts (though the app uses inline JS)
- No `X-Frame-Options` or `frame-ancestors` — admin UI can be framed (clickjacking)
- No `X-Content-Type-Options: nosniff`
- No `Strict-Transport-Security` (when HTTPS is enabled)
- No `Cache-Control` on sensitive responses (tokens, file listings)

**Impact:** Clickjacking attacks on admin interface, MIME-type confusion, cached sensitive data.

**Recommendation:**
- Add `X-Frame-Options: DENY`
- Add `X-Content-Type-Options: nosniff`
- Add `Cache-Control: no-store` on API responses
- Add CSP header (will require moving inline JS to external file)
- Add HSTS when HTTPS is enabled

---

### 10. Content-Disposition Header Injection — MEDIUM

**Location:** `src/handlers.rs:297` (serve_single_file), `src/handlers.rs:339` (serve_as_zip)

```rust
.content_disposition(&format!("attachment; filename=\"{filename}\""))
```

**Description:** The `filename` value is derived from the filesystem filename and is placed directly into the `Content-Disposition` header without sanitization. A filename containing `"` or `\r\n` could break the header or inject additional headers (HTTP response splitting).

**Impact:** HTTP response header injection, potential cache poisoning or XSS via reflected filename.

**Recommendation:**
- Sanitize filenames: remove or escape `"`, `\r`, `\n`
- Use RFC 6266 `filename*` encoding for non-ASCII names
- Restrict allowed characters in filenames

---

### 11. One-Time Enforcement Bypass via Config Endpoint — MEDIUM

**Location:** `src/handlers.rs:130-135` (routing), `src/handlers.rs:275-279` (config_one_time)

```rust
("GET", path) if path.starts_with("/config/one-time/") => { ... }
```

**Description:** Any user with access to the admin interface can disable one-time download enforcement by visiting `/config/one-time/false`. This is exposed as a simple GET endpoint with no confirmation, authentication, or CSRF protection. A single accidental or malicious request disables the core security feature.

**Impact:** Complete bypass of one-time download guarantee.

**Recommendation:**
- Require POST method with CSRF token
- Add authentication
- Log configuration changes prominently

---

### 12. UUID v4 Token Entropy — LOW

**Location:** `src/handlers.rs:228` 

```rust
let token = Uuid::new_v4().to_string();
```

**Description:** The `uuid` crate's v4 implementation uses the OS CSPRNG (`getrandom`), providing 122 bits of entropy. This is cryptographically adequate for the use case. However, tokens are stored indefinitely and never expire, so long-term brute-force accumulates.

**Impact:** Low — tokens are cryptographically random. The bigger risk is token leakage via the unauthenticated `/api/tokens` endpoint.

**Recommendation:**
- Add token expiration (TTL)
- Current UUID generation is acceptable for this use case

---

### 13. No CORS Policy — LOW

**Location:** All HTTP responses

**Description:** No `Access-Control-Allow-Origin` headers are set. While this means the admin API can't be accessed from cross-origin JavaScript (browser default is to block), the lack of explicit CORS also means:
- No protection against simple cross-origin requests (GET/POST with simple content types)
- The `/config/one-time/false` GET endpoint can be triggered via `<img src>` or `<script src>` from any page

**Impact:** CSRF-style attacks against configuration endpoints.

**Recommendation:**
- Add explicit CORS headers restricting to same-origin
- Convert state-changing operations to POST with CSRF tokens

---

### 14. No TLS Implementation — INFO

**Location:** `src/config.rs` (enable_https, cert_path, key_path fields), `src/server.rs` (no TLS code)

**Description:** The configuration has fields for TLS (`enable_https`, `cert_path`, `key_path`) but **TLS is never actually implemented** in the server code. Setting `enable_https = true` only changes the URL scheme in generated links — traffic is still plaintext.

**Impact:** False sense of security if a user enables the HTTPS config option. Download tokens are transmitted in plaintext.

**Recommendation:**
- Either implement TLS or remove the config options
- Add clear documentation that a reverse proxy is needed for HTTPS
- Warn at startup if `enable_https` is set but TLS is not actually active

---

### 15. No Connection Timeout / Slowloris — INFO

**Location:** `src/server.rs:115-140` (connection handling)

**Description:** The server reads from the socket with no timeout. A malicious client can open a connection and send data very slowly (Slowloris attack), holding the connection open indefinitely and eventually exhausting server resources.

**Impact:** Denial of service via connection exhaustion.

**Recommendation:**
- Add read timeouts on connections
- Limit maximum concurrent connections
- Consider using a production HTTP server library

---

## Summary Table

| # | Finding | Severity | CVSS-like |
|---|---------|----------|-----------|
| 1 | Unauthenticated admin on 0.0.0.0 | CRITICAL | 9.8 |
| 2 | One-time download race condition | CRITICAL | 8.1 |
| 3 | Stored XSS via filenames | CRITICAL | 8.4 |
| 4 | Path traversal via symlinks | HIGH | 7.5 |
| 5 | URL decode issues + path traversal | HIGH | 7.5 |
| 6 | Request truncation / no body validation | HIGH | 6.5 |
| 7 | DoS: unbounded tokens + OOM ZIP | HIGH | 7.5 |
| 8 | Information leakage (full paths) | MEDIUM | 5.3 |
| 9 | Missing security headers | MEDIUM | 5.0 |
| 10 | Content-Disposition header injection | MEDIUM | 5.4 |
| 11 | One-time enforcement bypass via GET | MEDIUM | 6.5 |
| 12 | UUID token entropy | LOW | 2.0 |
| 13 | No CORS policy / CSRF | LOW | 4.3 |
| 14 | TLS not implemented despite config | INFO | 3.0 |
| 15 | No connection timeout (Slowloris) | INFO | 5.3 |

---

## Recommendations Priority

1. **Immediate:** Bind admin to `127.0.0.1`, add authentication
2. **Immediate:** Fix race condition with `compare_exchange`
3. **Immediate:** Escape all user-controlled HTML output
4. **Short-term:** Canonicalize paths, reject symlinks
5. **Short-term:** Implement streaming responses, add size limits
6. **Medium-term:** Add security headers, CSRF protection, rate limiting
7. **Long-term:** Consider using an established HTTP framework (axum, actix-web) instead of hand-rolling HTTP parsing and serving
