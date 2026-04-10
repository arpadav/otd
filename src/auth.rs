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
    api::routes::auth::{LoginRequest, LoginResponse},
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
const COOKIE_NAME: &str = "otd_session";
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
fn extract_cookie_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .into_iter()
        .flat_map(|s| s.split(';'))
        .filter_map(|c| c.trim().strip_prefix(&format!("{COOKIE_NAME}=")))
        .next()
        .map(String::from)
}

/// Handles `POST /api/auth/login`
///
/// Verifies the password against the stored argon2 hash and sets an
/// `HttpOnly` session cookie on success. If no password is configured
/// (`admin_password_hash` is `None`), login succeeds unconditionally
pub async fn login(Json(req): Json<LoginRequest>) -> Result<Response, ApiError> {
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
    if !password_ok {
        return Ok(Json(LoginResponse { success: false }).into_response());
    }
    // --------------------------------------------------
    // release the config read lock before acquiring the
    // session store write lock
    // --------------------------------------------------
    drop(cfg);
    // --------------------------------------------------
    // generate random session token
    // --------------------------------------------------
    let session_token = generate_session_token()?;
    // --------------------------------------------------
    // store session server-side
    // --------------------------------------------------
    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + SESSION_MAX_AGE;
    SESSION_STORE.write().await.insert(
        session_token.clone(),
        Session {
            _user: ADMIN_USER.into(),
            expires_at: expiry,
        },
    );
    // --------------------------------------------------
    // build the cookie with the session token
    // --------------------------------------------------
    let cookie: axum::http::header::HeaderValue = Cookie::build((COOKIE_NAME, session_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(
            std::time::Duration::from_secs(SESSION_MAX_AGE)
                .try_into()
                .map_err(|_| ApiError::Internal("duration conversion failed".into()))?,
        )
        .build()
        .to_string()
        .parse()
        .map_err(|e: axum::http::header::InvalidHeaderValue| ApiError::Internal(e.to_string()))?;
    // --------------------------------------------------
    // insert the Set-Cookie header into the response
    // --------------------------------------------------
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
pub async fn logout(req: Request) -> Response {
    // --------------------------------------------------
    // remove session from store if present
    // --------------------------------------------------
    if let Some(token) = extract_cookie_token(req.headers()) {
        SESSION_STORE.write().await.remove(&token);
    }
    // --------------------------------------------------
    // clear the cookie
    // --------------------------------------------------
    let cookie_header = format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    let mut response = Json(LoginResponse { success: true }).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        #[allow(clippy::expect_used, reason = "cookie header is always valid ASCII")]
        cookie_header.parse().expect("valid cookie header"),
    );
    response
}

/// Auth middleware that validates the session cookie
///
/// Bypasses auth for:
/// - `POST /api/auth/login` (must work without auth)
/// - `GET /api/theme` (public endpoint for unauthenticated theme loading)
/// - All requests when no password is configured
pub async fn middleware(req: Request, next: Next) -> Result<Response, ApiError> {
    let path = req.uri().path();
    let method = req.method().clone();
    // --------------------------------------------------
    // bypass: login and public theme endpoints
    // --------------------------------------------------
    if path == "/auth/login" || (method == axum::http::Method::GET && path == "/theme") {
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
}
