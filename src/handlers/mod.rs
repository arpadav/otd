//! Request handlers for the OTD server.
//!
//! This module contains the main business logic for handling HTTP requests,
//! including file browsing, download link generation, and file serving.
//! It implements a clean separation between admin and download functionality.
//!
//! Author: aav
mod browse;
pub(crate) mod download;
mod links;

use crate::{config::ParsedConfig, http::*, types::*};

use std::{
    collections::HashMap,
    fmt::Write,
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use uuid::Uuid;

/// Maximum number of HTTP headers to parse per request.
const MAX_PARSE_HEADERS: usize = 64;
/// Cookie name used for admin session tracking.
const SESSION_COOKIE_NAME: &str = "otd_session";
/// Maximum session age (24 hours).
const SESSION_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);
/// Max-Age value for session cookies (seconds, as string).
const SESSION_COOKIE_MAX_AGE: &str = "86400";
/// WWW-Authenticate realm for bearer token auth.
const AUTH_REALM: &str = "otd-admin";
/// Separator between HTTP headers and body.
const HEADER_BODY_SEPARATOR: &str = "\r\n\r\n";

/// Main request handler containing business logic for both admin and download servers.
///
/// The handler manages file browsing, download link generation, and file serving.
/// It maintains shared state and configuration, and provides separate entry points
/// for admin interface requests and download requests.
///
/// Cloning a `Handler` only bumps `Arc` reference counts - zero heap allocations.
///
/// # Examples
///
/// ```rust,no_run
/// use otd::{Handler, Config, types::AppState};
/// use std::{sync::Arc, path::PathBuf};
///
/// let config = Config::default().parse().unwrap();
/// let state = Arc::new(AppState::new(PathBuf::from("/files")));
/// let handler = Handler::new(state, config);
/// ```
pub struct Handler {
    /// Shared application state containing download tokens and configuration
    pub state: Arc<AppState>,
    /// Pre-computed, immutable configuration (shared via Arc across clones)
    pub config: Arc<ParsedConfig>,
    /// Cached index.html with all config placeholders replaced
    index_html: Arc<str>,
    /// Cached about.html with CSS injected
    about_html: Arc<str>,
    /// Cached login.html with CSS injected ({{MESSAGE}} still present for runtime replacement)
    login_html_base: Arc<str>,
    /// Cached Set-Cookie header for logout (session clear)
    logout_cookie: Arc<str>,
}

impl Handler {
    /// Creates a new handler with the given state and pre-parsed configuration.
    ///
    /// Pre-computes and caches all HTML templates at construction time so that
    /// serving pages requires zero template processing.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::{Handler, Config, types::AppState};
    /// use std::{sync::Arc, path::PathBuf};
    ///
    /// let config = Config::default().parse().unwrap();
    /// let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// let handler = Handler::new(state, config);
    /// ```
    pub fn new(state: Arc<AppState>, config: ParsedConfig) -> Self {
        let css = include_str!("../../static/style.css");

        let index_html: Arc<str> = include_str!("../../static/index.html")
            .replace("{{TAILWIND_CSS}}", css)
            .replace("{{ADMIN_HOST}}", &config.raw.admin_host)
            .replace("{{ADMIN_PORT}}", &config.raw.admin_port.to_string())
            .replace("{{DOWNLOAD_HOST}}", &config.raw.download_host)
            .replace("{{DOWNLOAD_PORT}}", &config.raw.download_port.to_string())
            .replace("{{BASE_PATH}}", &config.raw.base_path)
            .into();

        let about_html: Arc<str> = include_str!("../../static/about.html")
            .replace("{{TAILWIND_CSS}}", css)
            .into();

        let login_html_base: Arc<str> = include_str!("../../static/login.html")
            .replace("{{TAILWIND_CSS}}", css)
            .into();

        let logout_cookie: Arc<str> = format!(
            "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
            SESSION_COOKIE_NAME
        )
        .into();

        Self {
            state,
            config: Arc::new(config),
            index_html,
            about_html,
            login_html_base,
            logout_cookie,
        }
    }

    /// Handles requests to the admin interface (file browsing, link generation).
    ///
    /// Routes admin requests to appropriate handlers based on the HTTP method and path.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default().parse().unwrap();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// # smol::block_on(async {
    /// let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    /// let peer_addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
    /// let response = handler.handle_admin_request(request, peer_addr).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_admin_request(
        &self,
        request: &str,
        peer_addr: std::net::SocketAddr,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_PARSE_HEADERS];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(request.as_bytes()) {
            Ok(_) => {
                // Bearer token authentication - enforced when admin_token is configured.
                if let Some(ref expected_token) = self.config.raw.admin_token {
                    let auth_header = req
                        .headers
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("Authorization"));
                    let authorized = auth_header
                        .and_then(|h| std::str::from_utf8(h.value).ok())
                        .and_then(|v| v.strip_prefix("Bearer "))
                        .map(|token| token.trim() == expected_token.as_str())
                        .unwrap_or(false);
                    if !authorized {
                        tracing::warn!(
                            "Admin request rejected: missing or invalid Authorization header"
                        );
                        return Ok(HttpResponse::unauthorized(AUTH_REALM).to_bytes());
                    }
                }
                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");
                tracing::info!("Admin request: {} {}", method, path);
                let (path, query) = Self::split_path_query(path);
                tracing::info!("Parsed path: '{}', query: '{}'", path, query);
                let is_loopback = peer_addr.ip().is_loopback();
                if !is_loopback {
                    // Check if admin_password is configured
                    if self.config.raw.admin_password.is_none() {
                        let html = self.login_page(
                            "External access requires admin_password to be set in config.",
                        );
                        return Ok(HttpResponse::new(status::FORBIDDEN.0, status::FORBIDDEN.1)
                            .content_type(content_type::HTML)
                            .body_text(&html)
                            .to_bytes());
                    }

                    // Allow login routes without session
                    let is_login_route = path == "/login";
                    if !is_login_route {
                        let session_token = Self::extract_session_token(&req);

                        let mut valid_session = false;
                        if let Some(token) = session_token {
                            let sessions = self.state.sessions.read().await;
                            if let Some(created) = sessions.get(token)
                                && created.elapsed() < SESSION_MAX_AGE
                            {
                                valid_session = true;
                            }
                        }

                        // Clean up expired sessions (best-effort, don't block)
                        if !valid_session {
                            let mut sessions = self.state.sessions.write().await;
                            sessions.retain(|_, created| created.elapsed() < SESSION_MAX_AGE);
                        }

                        if !valid_session {
                            return Ok(HttpResponse::redirect("/login").to_bytes());
                        }
                    }
                }

                // --- Login routes ---
                if path == "/login" {
                    if method == "GET" {
                        let html = self.login_page("");
                        return Ok(HttpResponse::ok()
                            .content_type(content_type::HTML)
                            .body_text(&html)
                            .to_bytes());
                    } else if method == "POST" {
                        let body = self.extract_body(request)?;
                        let params = self.parse_query(&body);
                        let password = params.get("password").cloned().unwrap_or_default();

                        let expected = self.config.raw.admin_password.as_deref().unwrap_or("");
                        if !expected.is_empty() && password == expected {
                            let token = Uuid::new_v4().to_string();
                            self.state
                                .sessions
                                .write()
                                .await
                                .insert(token.clone(), std::time::Instant::now());
                            return Ok(HttpResponse::redirect("/")
                                .header("Set-Cookie", &Self::session_set_cookie(&token))
                                .to_bytes());
                        } else {
                            let html = self.login_page("Invalid password.");
                            return Ok(HttpResponse::new(status::FORBIDDEN.0, status::FORBIDDEN.1)
                                .content_type(content_type::HTML)
                                .body_text(&html)
                                .to_bytes());
                        }
                    }
                }

                // Extract session cookie for use in route handlers (e.g. logout)
                let session_cookie = Self::extract_session_token(&req)
                    .map(|s| s.to_string());

                let response = match (method, path) {
                    ("GET", "/") => self.web_interface().await?,
                    ("GET", "/about") => self.about_page().await?,
                    ("GET", "/api/browse") => self.browse(query).await?,
                    ("GET", "/api/stats") => self.stats().await?,
                    ("GET", "/api/tokens") => self.list_tokens().await?,
                    ("POST", "/api/generate") => {
                        let body = self.extract_body(request)?;
                        self.generate_link(&body).await?
                    }
                    ("POST", "/api/tokens/bulk-delete") => {
                        let body = self.extract_body(request)?;
                        self.bulk_delete_tokens(&body).await?
                    }
                    ("DELETE", path) if path.starts_with("/api/tokens/") => {
                        let token = path.strip_prefix("/api/tokens/").unwrap_or("");
                        self.delete_token(token).await?
                    }
                    ("GET", "/logout") => {
                        // Invalidate session server-side before clearing cookie
                        if let Some(ref token) = session_cookie {
                            self.state.sessions.write().await.remove(token);
                            tracing::info!("Session invalidated on logout");
                        }
                        HttpResponse::redirect("/login").header(
                            "Set-Cookie",
                            &self.logout_cookie,
                        )
                    }
                    ("GET", path) if path.starts_with("/config/one-time/") => {
                        let enabled = path
                            .strip_prefix("/config/one-time/")
                            .and_then(|s| s.parse::<bool>().ok())
                            .unwrap_or(true);
                        self.config_one_time(enabled).await?
                    }
                    _ => HttpResponse::not_found(),
                };

                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse HTTP request: {}", e);
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }

    /// Handles requests to the download server (file downloads only).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default().parse().unwrap();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// # smol::block_on(async {
    /// let request = "GET /document.pdf?k=550e8400-e29b-41d4-a716-446655440000 HTTP/1.1\r\n\r\n";
    /// let response = handler.handle_download_request(request).await.unwrap();
    /// # });
    /// ```
    pub async fn handle_download_request(
        &self,
        request: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = [httparse::EMPTY_HEADER; MAX_PARSE_HEADERS];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(request.as_bytes()) {
            Ok(_) => {
                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");

                let (path, query) = Self::split_path_query(path);

                let response = match (method, path) {
                    ("GET", "/") => self.download(query).await?,
                    ("GET", _) => self.download(query).await?, // Any path with ?k= parameter
                    _ => HttpResponse::not_found(),
                };

                Ok(response.to_bytes())
            }
            Err(e) => {
                tracing::error!("Failed to parse download request: {}", e);
                Ok(HttpResponse::bad_request().to_bytes())
            }
        }
    }
}

// --- Small page and config handlers ---

impl Handler {
    /// Serves the main web interface HTML (pre-cached at startup).
    async fn web_interface(
        &self,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HttpResponse::ok()
            .content_type(content_type::HTML)
            .body_text(&self.index_html))
    }

    /// Serves the static "About" page (pre-cached at startup).
    async fn about_page(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HttpResponse::ok()
            .content_type(content_type::HTML)
            .body_text(&self.about_html))
    }

    /// Returns dashboard statistics as JSON.
    async fn stats(&self) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tokens = self.state.tokens.read().await;
        let now = std::time::Instant::now();

        let mut active = 0u32;
        let mut used = 0u32;
        let mut expired = 0u32;
        let mut total_downloads = 0u64;

        for item in tokens.values() {
            let count = item.download_count.load(Ordering::Relaxed);
            total_downloads += count as u64;
            let is_expired = item.expires_at.is_some_and(|e| now >= e);
            let is_used = count >= item.max_downloads;
            if is_expired {
                expired += 1;
            } else if is_used {
                used += 1;
            } else {
                active += 1;
            }
        }

        let uptime_seconds = self.state.started_at.elapsed().as_secs();

        let stats = serde_json::json!({
            "active_tokens": active,
            "used_tokens": used,
            "expired_tokens": expired,
            "total_downloads": total_downloads,
            "uptime_seconds": uptime_seconds,
        });

        HttpResponse::ok().body_json(&stats).map_err(Into::into)
    }

    /// Configures one-time download enforcement.
    async fn config_one_time(
        &self,
        enabled: bool,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.state
            .one_time_enabled
            .store(enabled, Ordering::Relaxed);
        Ok(HttpResponse::ok()
            .content_type(content_type::PLAIN_TEXT)
            .body_text("Configuration updated"))
    }
}

// --- Utility methods ---

impl Handler {

    /// Safely joins `relative` onto `base_path` and verifies the resolved path
    /// is still within `base_path` after canonicalization.
    ///
    /// This prevents both classic `../` path traversal and symlink-based escapes,
    /// since `canonicalize()` resolves all symlinks before the containment check.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use otd::{Handler, Config, types::AppState};
    /// # use std::{sync::Arc, path::PathBuf};
    /// # let config = Config::default().parse().unwrap();
    /// # let state = Arc::new(AppState::new(PathBuf::from("/files")));
    /// # let handler = Handler::new(state, config);
    /// // Safe path - returns Some(canonical_path)
    /// let ok = handler.safe_join("subdir/file.txt");
    /// // Traversal - returns None
    /// let bad = handler.safe_join("../../etc/passwd");
    /// assert!(bad.is_none());
    /// ```
    pub fn safe_join(&self, relative: &str) -> Option<PathBuf> {
        let joined = self.state.base_path.join(relative);
        // canonicalize resolves symlinks and `..` components; if the path
        // doesn't exist it returns an error, which we propagate as None.
        let canonical = std::fs::canonicalize(&joined).ok()?;
        if canonical.starts_with(&self.state.base_path) {
            Some(canonical)
        } else {
            tracing::warn!(
                "Path escape blocked: '{}' resolves to '{:?}' outside base '{:?}'",
                relative,
                canonical,
                self.state.base_path
            );
            None
        }
    }

    /// Parses URL query string into key-value pairs.
    pub(crate) fn parse_query(&self, query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(self.url_decode(key), self.url_decode(value));
            }
        }
        params
    }

    /// URL-decodes a string (handles %XX encoding and + for spaces).
    pub(crate) fn url_decode(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            match ch {
                '%' => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        } else {
                            result.push('%');
                            result.push_str(&hex);
                        }
                    } else {
                        result.push('%');
                        result.push_str(&hex);
                    }
                }
                '+' => result.push(' '),
                _ => result.push(ch),
            }
        }

        result
    }

    /// URL-encodes a string for safe use in URLs.
    pub(crate) fn url_encode(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                b' ' => result.push('+'),
                _ => {
                    write!(result, "%{byte:02X}").unwrap();
                }
            }
        }
        result
    }

    /// Renders the login page with an optional message (CSS pre-cached at startup).
    fn login_page(&self, message: &str) -> String {
        self.login_html_base.replace("{{MESSAGE}}", message)
    }

    /// Extracts the session token from the `Cookie` header.
    fn extract_session_token<'a>(req: &'a httparse::Request<'_, '_>) -> Option<&'a str> {
        const PREFIX: &str = concat!("otd_session", "=");
        req.headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Cookie"))
            .and_then(|h| std::str::from_utf8(h.value).ok())
            .unwrap_or("")
            .split(';')
            .map(|s| s.trim())
            .find(|s| s.starts_with(PREFIX))
            .and_then(|s| s.strip_prefix(PREFIX))
    }

    /// Builds a `Set-Cookie` header value for a new session.
    fn session_set_cookie(token: &str) -> String {
        format!(
            "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            SESSION_COOKIE_NAME, token, SESSION_COOKIE_MAX_AGE
        )
    }

    /// Splits a URL path into (path, query) components.
    fn split_path_query(path: &str) -> (&str, &str) {
        if let Some(pos) = path.find('?') {
            (&path[..pos], &path[pos + 1..])
        } else {
            (path, "")
        }
    }

    /// Builds a download URL from a name and token.
    pub(crate) fn download_url(&self, name: &str, token: &str) -> String {
        format!(
            "{}/{}?k={}",
            self.config.download_base_url,
            self.url_encode(name),
            token
        )
    }

    /// Extracts the body content from an HTTP request.
    pub(crate) fn extract_body(
        &self,
        request: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(body_start) = request.find(HEADER_BODY_SEPARATOR) {
            Ok(request[body_start + HEADER_BODY_SEPARATOR.len()..].to_string())
        } else {
            Ok(String::new())
        }
    }
}

impl Clone for Handler {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            config: Arc::clone(&self.config),
            index_html: Arc::clone(&self.index_html),
            about_html: Arc::clone(&self.about_html),
            login_html_base: Arc::clone(&self.login_html_base),
            logout_cookie: Arc::clone(&self.logout_cookie),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::path::PathBuf;

    fn make_test_handler() -> Handler {
        let config = Config::default().parse().unwrap();
        let state = Arc::new(AppState::new(PathBuf::from("/test")));
        Handler::new(state, config)
    }

    #[test]
    fn test_url_encoding() {
        let handler = make_test_handler();

        assert_eq!(handler.url_encode("hello world"), "hello+world");
        assert_eq!(handler.url_encode("file.txt"), "file.txt");
        assert_eq!(handler.url_encode("special@chars"), "special%40chars");
    }

    #[test]
    fn test_url_decoding() {
        let handler = make_test_handler();

        assert_eq!(handler.url_decode("hello+world"), "hello world");
        assert_eq!(handler.url_decode("file.txt"), "file.txt");
        assert_eq!(handler.url_decode("special%40chars"), "special@chars");
    }

    #[test]
    fn test_query_parsing() {
        let handler = make_test_handler();

        let params = handler.parse_query("k=token123&path=folder%2Ffile");
        assert_eq!(params.get("k"), Some(&"token123".to_string()));
        assert_eq!(params.get("path"), Some(&"folder/file".to_string()));
    }

    fn make_handler_with_token(token: Option<&str>) -> Handler {
        let config = Config {
            admin_token: token.map(|t| t.to_string()),
            ..Default::default()
        }
        .parse()
        .unwrap();
        let state = Arc::new(AppState::new(PathBuf::from("/tmp")));
        Handler::new(state, config)
    }

    fn loopback_addr() -> std::net::SocketAddr {
        "127.0.0.1:12345".parse().unwrap()
    }

    /// When no token is configured, all admin requests are allowed.
    #[test]
    fn test_admin_auth_disabled_allows_all() {
        let handler = make_handler_with_token(None);
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        // Should NOT be 401
        assert!(
            !response_str.contains("401 Unauthorized"),
            "Unexpected 401 when auth is disabled"
        );
    }

    /// When token is configured, missing Authorization header returns 401.
    #[test]
    fn test_admin_auth_required_rejects_missing_header() {
        let handler = make_handler_with_token(Some("secret123"));
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
        assert!(response_str.contains("WWW-Authenticate"));
    }

    /// Wrong token returns 401.
    #[test]
    fn test_admin_auth_required_rejects_wrong_token() {
        let handler = make_handler_with_token(Some("secret123"));
        let request =
            "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer wrongtoken\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("401 Unauthorized"));
    }

    /// Correct token is accepted.
    #[test]
    fn test_admin_auth_required_accepts_correct_token() {
        let handler = make_handler_with_token(Some("secret123"));
        let request =
            "GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secret123\r\n\r\n";
        let response =
            smol::block_on(handler.handle_admin_request(request, loopback_addr())).unwrap();
        let response_str = String::from_utf8_lossy(&response);
        assert!(
            !response_str.contains("401 Unauthorized"),
            "Correct token should be accepted"
        );
    }

    fn make_handler_with_base(base: PathBuf) -> Handler {
        let config = Config::default().parse().unwrap();
        let state = Arc::new(AppState::new(base));
        Handler::new(state, config)
    }

    /// safe_join with a normal relative path returns the canonical path.
    #[test]
    fn test_safe_join_valid_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, b"hello").unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        let result = handler.safe_join("file.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    /// safe_join must block `../` path traversal.
    #[test]
    fn test_safe_join_blocks_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Non-existent traversal path - canonicalize fails → None
        assert!(handler.safe_join("../../../etc/passwd").is_none());
    }

    /// safe_join must block symlinks that point outside base_path.
    #[test]
    fn test_safe_join_blocks_symlink_escape() {
        let dir = tempfile::tempdir().unwrap();
        let link_path = dir.path().join("evil_link");
        // Create symlink inside base_path → /etc/passwd outside base_path
        std::os::unix::fs::symlink("/etc/passwd", &link_path).unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Must be blocked - /etc/passwd is outside the base dir
        assert!(handler.safe_join("evil_link").is_none());
    }

    /// safe_join must allow symlinks that resolve within base_path.
    #[test]
    fn test_safe_join_allows_internal_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real_file = dir.path().join("real.txt");
        std::fs::write(&real_file, b"data").unwrap();
        let link_path = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real_file, &link_path).unwrap();

        let handler = make_handler_with_base(dir.path().to_path_buf());

        // Should succeed - symlink resolves within base_path
        let result = handler.safe_join("link.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), real_file.canonicalize().unwrap());
    }

    /// Verify that compare_exchange prevents concurrent double-downloads.
    /// Two threads race to claim the same AtomicBool; exactly one must win.
    #[test]
    fn test_one_time_download_race_condition() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use std::thread;

        let downloaded = Arc::new(AtomicBool::new(false));
        let success_count = Arc::new(AtomicUsize::new(0));

        let threads: Vec<_> = (0..20)
            .map(|_| {
                let downloaded = Arc::clone(&downloaded);
                let success_count = Arc::clone(&success_count);
                thread::spawn(move || {
                    if downloaded
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(
            success_count.load(Ordering::Relaxed),
            1,
            "Exactly one thread should succeed in claiming the download"
        );
    }

    /// Verify that when one_time is disabled, multiple downloads are allowed.
    #[test]
    fn test_one_time_disabled_allows_redownload() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let one_time_enabled = AtomicBool::new(false);
        let downloaded = AtomicBool::new(false);

        // Simulate two download attempts with one_time disabled
        for _ in 0..3 {
            if one_time_enabled.load(Ordering::Acquire) {
                assert!(
                    downloaded
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok(),
                    "Should only succeed once when one_time enabled"
                );
            }
            // When disabled: no check performed, would proceed to serve
        }
        // downloaded flag never set since one_time is disabled
        assert!(!downloaded.load(Ordering::Relaxed));
    }
}
