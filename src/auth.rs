//! Cookie-based session authentication using argon2 password hashing
//!
//! Sessions are stored server-side in [`SESSION_STORE`] and referenced by
//! a random token set as an `HttpOnly` cookie. Passwords are verified
//! against an argon2 hash stored in [`PersistentConfig`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    api::error::ApiError,
    api::routes::auth::{LoginRequest, LoginResponse, MeResponse},
    config,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use argon2::PasswordVerifier;
use axum::{
    Json,
    extract::Request,
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::Engine;
use ring::rand::{SecureRandom, SystemRandom};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Cookie name for the session token
pub(crate) const COOKIE_NAME: &str = "otd_session";
/// Session lifetime in seconds (24 hours)
const SESSION_MAX_AGE: u64 = 86400;
/// Admin username for sessions
const ADMIN_USER: &str = "admin";
/// How often expired sessions are cleaned up
const SESSION_CLEANUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(300);

/// Server-side session entry
pub(crate) struct Session {
    /// Username associated with this session
    _user: String,
    /// When this session expires (unix timestamp seconds)
    expires_at: u64,
}

/// In-memory session store mapping session tokens to sessions
pub(crate) static SESSION_STORE: std::sync::LazyLock<
    tokio::sync::RwLock<std::collections::HashMap<String, Session>>,
> = std::sync::LazyLock::new(|| tokio::sync::RwLock::new(std::collections::HashMap::new()));

/// Removes expired sessions from the store
///
/// Acquires a write lock on [`SESSION_STORE`] and retains only entries whose
/// `expires_at` timestamp is in the future relative to the current unix time
/// Called periodically from the session cleanup background task
pub(crate) async fn cleanup_sessions() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    SESSION_STORE
        .write()
        .await
        .retain(|_, s| s.expires_at > now);
}

/// Spawns the periodic session cleanup background task
///
/// Loops indefinitely, sleeping for [`SESSION_CLEANUP_INTERVAL`] between
/// each call to [`cleanup_sessions`]. Runs as a detached tokio task
pub(crate) fn spawn_session_cleanup() {
    tokio::spawn(async {
        loop {
            tokio::time::sleep(SESSION_CLEANUP_INTERVAL).await;
            cleanup_sessions().await;
        }
    });
}

/// Generates a cryptographically random 32-byte token and base64url-encodes it
///
/// Uses [`ring::rand::SystemRandom`] to fill a 32-byte buffer, then encodes
/// it with [`base64::engine::general_purpose::URL_SAFE_NO_PAD`]. Returns an
/// [`ApiError::Internal`] if the OS RNG fails
fn generate_session_token() -> Result<String, ApiError> {
    let rng = SystemRandom::new();
    let mut dst = [0_u8; 32];
    rng.fill(&mut dst)
        .map_err(|_| ApiError::Internal("failed to generate session token".into()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(dst))
}

/// Extracts the session token value from the `Cookie` request header
///
/// Parses the raw `Cookie` header, splits on `;`, and finds the first entry
/// prefixed with `{COOKIE_NAME}=`. Returns `None` if the header is absent,
/// not valid UTF-8, or does not contain the session cookie
///
/// # Arguments
///
/// * `headers` - The request header map to search
pub(crate) fn extract_cookie_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .into_iter()
        .flat_map(|s| s.split(';'))
        .filter_map(|c| c.trim().strip_prefix(&format!("{COOKIE_NAME}=")))
        .next()
        .map(String::from)
}

/// Detects whether the original client request was made over HTTPS
///
/// Inspects `X-Forwarded-Proto` (set by reverse proxies like nginx/caddy)
/// and falls back to the request URI scheme when terminating TLS directly
/// Returns `false` for plain HTTP, which is the common local-network and
/// mobile-on-LAN case where a `Secure`-flagged cookie would be silently
/// dropped by the browser
pub(crate) fn request_is_https(headers: &axum::http::HeaderMap, uri: &axum::http::Uri) -> bool {
    if let Some(proto) = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
    {
        return proto
            .split(',')
            .next()
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("https"));
    }
    uri.scheme_str() == Some("https")
}

/// Builds a `Set-Cookie` header value for the session cookie
///
/// `secure` controls the `Secure` attribute. `max_age_secs` of `None` clears
/// the cookie (used by logout); `Some(seconds)` sets a fresh expiry. The
/// cookie is always `HttpOnly`, `SameSite=Strict`, and scoped to `/`
fn build_session_cookie_header(
    token: &str,
    secure: bool,
    max_age_secs: Option<u64>,
) -> Result<axum::http::header::HeaderValue, ApiError> {
    let dur = match max_age_secs {
        Some(secs) => std::time::Duration::from_secs(secs),
        None => std::time::Duration::ZERO,
    };
    let cookie = Cookie::build((COOKIE_NAME, token.to_string()))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(
            dur.try_into()
                .map_err(|_| ApiError::Internal("duration conversion failed".into()))?,
        )
        .build();
    cookie
        .to_string()
        .parse()
        .map_err(|e: axum::http::header::InvalidHeaderValue| ApiError::Internal(e.to_string()))
}

/// Builds a `Set-Cookie` header value for an issued session token
///
/// Convenience for handlers outside this module (e.g. password change) that
/// need to set the session cookie on their response. `secure` should be the
/// result of [`request_is_https`] for the originating request
pub(crate) fn session_cookie_header(
    token: &str,
    secure: bool,
) -> Result<axum::http::header::HeaderValue, ApiError> {
    build_session_cookie_header(token, secure, Some(SESSION_MAX_AGE))
}

/// Removes `old_token` from the session store (if any) and inserts a freshly
/// generated session. Returns the new token so the caller can set it as a
/// cookie on the outgoing response
pub(crate) async fn rotate_session(old_token: Option<&str>) -> Result<String, ApiError> {
    let new_token = generate_session_token()?;
    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + SESSION_MAX_AGE;
    let mut store = SESSION_STORE.write().await;
    if let Some(t) = old_token {
        store.remove(t);
    }
    store.insert(
        new_token.clone(),
        Session {
            _user: ADMIN_USER.into(),
            expires_at: expiry,
        },
    );
    Ok(new_token)
}

/// Handles `POST /api/auth/login`
///
/// Verifies the password against the stored argon2 hash and sets an
/// `HttpOnly` session cookie on success. If no password is configured
/// (`admin_password_hash` is `None`), login succeeds unconditionally
pub async fn login(
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
    Json(req): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let secure = request_is_https(&headers, &uri);
    let cfg = config::config().read().await;
    // --------------------------------------------------
    // check password with argon2
    // --------------------------------------------------
    let password_ok = match &cfg.persistent.admin_password_hash {
        Some(stored_hash) => {
            let parsed_hash = argon2::PasswordHash::new(stored_hash)
                .map_err(|_| ApiError::Internal("invalid stored password hash".into()))?;
            argon2::Argon2::default()
                .verify_password(req.password.as_bytes(), &parsed_hash)
                .is_ok()
        }
        None => true,
    };
    drop(cfg);
    if !password_ok {
        return Ok(Json(LoginResponse { success: false }).into_response());
    }
    // --------------------------------------------------
    // generate and store the session, then set the cookie
    // --------------------------------------------------
    let token = rotate_session(None).await?;
    let cookie = session_cookie_header(&token, secure)?;
    let mut response = Json(LoginResponse { success: true }).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

/// Handles `POST /api/auth/logout`
///
/// Invalidates the server-side session by removing it from [`SESSION_STORE`]
/// if a valid session cookie is present. Always responds with a `Set-Cookie`
/// header that clears the cookie client-side regardless of whether a session
/// was found
pub async fn logout(req: Request) -> Result<Response, ApiError> {
    let secure = request_is_https(req.headers(), req.uri());
    if let Some(token) = extract_cookie_token(req.headers()) {
        SESSION_STORE.write().await.remove(&token);
    }
    let cookie = build_session_cookie_header("", secure, None)?;
    let mut response = Json(LoginResponse { success: true }).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

/// Handles `GET /api/auth/me`
///
/// Reports whether the request carries a valid session cookie and whether
/// the server has a password configured. Used by the frontend on mount to
/// determine whether to render the authenticated shell or redirect to login
/// without going through the side effect of any business endpoint
pub async fn me(req: Request) -> Json<MeResponse> {
    let password_required = config::config()
        .read()
        .await
        .persistent
        .admin_password_hash
        .is_some();
    if !password_required {
        return Json(MeResponse {
            logged_in: true,
            password_required: false,
        });
    }
    let logged_in = match extract_cookie_token(req.headers()) {
        Some(token) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            SESSION_STORE
                .read()
                .await
                .get(&token)
                .is_some_and(|s| s.expires_at > now)
        }
        None => false,
    };
    Json(MeResponse {
        logged_in,
        password_required,
    })
}

/// Auth middleware that validates the session cookie
///
/// Bypasses auth for:
/// - `POST /api/auth/login` (must work without auth)
/// - `POST /api/auth/logout` (idempotent; safe even without a session)
/// - `GET /api/auth/me` (probe endpoint reports auth state itself)
/// - `GET /api/theme` (public endpoint for unauthenticated theme loading)
/// - All requests when no password is configured
pub async fn middleware(req: Request, next: Next) -> Result<Response, ApiError> {
    let path = req.uri().path();
    let method = req.method().clone();
    // --------------------------------------------------
    // bypass: auth + public endpoints
    // --------------------------------------------------
    if path == "/auth/login"
        || path == "/auth/logout"
        || (method == axum::http::Method::GET && path == "/auth/me")
        || (method == axum::http::Method::GET && path == "/theme")
    {
        return Ok(next.run(req).await);
    }
    // --------------------------------------------------
    // bypass: no password configured (read lock must be
    // dropped before calling next.run so handlers can
    // acquire a write lock without deadlocking)
    // --------------------------------------------------
    let no_password = config::config()
        .read()
        .await
        .persistent
        .admin_password_hash
        .is_none();
    if no_password {
        return Ok(next.run(req).await);
    }
    // --------------------------------------------------
    // extract session token from cookie
    // --------------------------------------------------
    let token = extract_cookie_token(req.headers()).ok_or(ApiError::Unauthorized)?;
    // --------------------------------------------------
    // validate session in store
    // --------------------------------------------------
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let store = SESSION_STORE.read().await;
    match store.get(&token) {
        Some(session) if session.expires_at > now => {
            drop(store);
            Ok(next.run(req).await)
        }
        Some(_) => {
            // --------------------------------------------------
            // session expired - remove it
            // --------------------------------------------------
            drop(store);
            SESSION_STORE.write().await.remove(&token);
            Err(ApiError::Unauthorized)
        }
        None => Err(ApiError::Unauthorized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_hash_verify_roundtrip() {
        use argon2::PasswordHasher;
        let password = "test-password-123";
        let hash = argon2::Argon2::default()
            .hash_password(password.as_bytes())
            .expect("hash_password should succeed")
            .to_string();
        let parsed = argon2::PasswordHash::new(&hash).expect("should parse PHC string");
        assert!(
            argon2::Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
            "correct password should verify"
        );
        assert!(
            argon2::Argon2::default()
                .verify_password(b"wrong-password", &parsed)
                .is_err(),
            "wrong password should fail"
        );
    }

    #[tokio::test]
    async fn rotate_session_replaces_old_token() {
        let initial = rotate_session(None).await.expect("create initial session");
        assert!(SESSION_STORE.read().await.contains_key(&initial));
        let next = rotate_session(Some(&initial))
            .await
            .expect("rotate session");
        let store = SESSION_STORE.read().await;
        assert!(!store.contains_key(&initial), "old token must be removed");
        assert!(store.contains_key(&next), "new token must be present");
    }

    #[test]
    fn https_detection_prefers_x_forwarded_proto() {
        let mut headers = axum::http::HeaderMap::new();
        let uri: axum::http::Uri = "/api/foo".parse().expect("uri");
        assert!(!request_is_https(&headers, &uri));
        headers.insert("x-forwarded-proto", "https".parse().expect("hv"));
        assert!(request_is_https(&headers, &uri));
        headers.insert("x-forwarded-proto", "http".parse().expect("hv"));
        assert!(!request_is_https(&headers, &uri));
    }

    #[test]
    fn cookie_header_omits_secure_on_http() {
        let header = build_session_cookie_header("abc", false, Some(60)).expect("build");
        let s = header.to_str().expect("ascii");
        assert!(!s.to_ascii_lowercase().contains("secure"), "got: {s}");
    }

    #[test]
    fn cookie_header_includes_secure_on_https() {
        let header = build_session_cookie_header("abc", true, Some(60)).expect("build");
        let s = header.to_str().expect("ascii");
        assert!(s.to_ascii_lowercase().contains("secure"), "got: {s}");
    }
}
