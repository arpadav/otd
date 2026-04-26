//! Settings management endpoints
//!
//! Provides API endpoints for reading and updating persistent server
//! configuration (download base URL, admin password)
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::config;

// --------------------------------------------------
// external
// --------------------------------------------------
use argon2::PasswordVerifier;
use axum::{
    Json,
    extract::Request,
    http::header,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

// --------------------------------------------------
// types
// --------------------------------------------------
#[derive(Debug, Serialize, Deserialize)]
/// Response payload for `GET /api/settings`
pub struct SettingsResponse {
    /// Custom download base URL (`None` = derived from CLI host/port)
    pub download_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Request payload for `PUT /api/settings`
pub struct UpdateSettingsRequest {
    /// New download base URL (`None` to reset to default)
    pub download_base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Request payload for `POST /api/settings/password`
pub struct ChangePasswordRequest {
    /// Current admin password (empty string if no password is set)
    pub old_password: String,
    /// New admin password to set
    pub new_password: String,
}

/// Handles `GET /api/settings`
///
/// Reads the current persistent configuration and returns it as JSON
///
/// At present the only exposed setting is `download_base_url`; a `None`
/// value means the server derives the URL from its CLI host and port
pub async fn get_settings() -> Json<SettingsResponse> {
    // --------------------------------------------------
    // read the current config and extract persistent fields
    // --------------------------------------------------
    let cfg = config::config().read().await;
    Json(SettingsResponse {
        download_base_url: cfg.persistent.download_base_url.clone(),
    })
}

/// Handles `PUT /api/settings`
///
/// Applies the requested changes to persistent configuration, writes the
/// updated config to disk, and refreshes any derived in-memory values
/// (e.g. the resolved download base URL). Returns the updated settings on
/// success, or 500 Internal Server Error if the disk write fails
///
/// # Arguments
///
/// * `req` - JSON body deserialized into [`UpdateSettingsRequest`]
pub async fn update_settings(
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<SettingsResponse>, ApiError> {
    // --------------------------------------------------
    // acquire a write lock and apply the new setting
    // --------------------------------------------------
    let mut cfg = config::config().write().await;
    cfg.persistent.download_base_url = req.download_base_url;
    // --------------------------------------------------
    // persist the updated config to disk
    // --------------------------------------------------
    cfg.persistent
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    // --------------------------------------------------
    // refresh derived in-memory values that depend on the
    // updated persistent config
    // --------------------------------------------------
    cfg.refresh_download_base_url();
    Ok(Json(SettingsResponse {
        download_base_url: cfg.persistent.download_base_url.clone(),
    }))
}

/// Handles `POST /api/settings/password`
///
/// Verifies the currently stored admin password (if one exists), hashes the
/// new password with argon2 outside any lock, then updates the stored hash
/// under a write lock with a compare-and-swap so concurrent password changes
/// can't silently overwrite each other
///
/// On success, every existing session is invalidated except the caller's,
/// which is rotated to a fresh token returned in a `Set-Cookie` header so
/// the user stays logged in seamlessly
///
/// Returns 401 Unauthorized if the old password does not match (or the hash
/// changed under us), or 500 if hashing or the disk write fails
pub async fn change_password(req: Request) -> Result<Response, ApiError> {
    // --------------------------------------------------
    // capture request metadata before consuming the body
    // --------------------------------------------------
    let secure = crate::auth::request_is_https(req.headers(), req.uri());
    let caller_token = crate::auth::extract_cookie_token(req.headers());
    let (_, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 64 * 1024)
        .await
        .map_err(|e| ApiError::BadRequest(format!("failed to read body: {e}")))?;
    let payload: ChangePasswordRequest = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON body: {e}")))?;
    // --------------------------------------------------
    // verify old password under a read lock and snapshot
    // the current hash so we can compare-and-swap below
    // --------------------------------------------------
    let prior_hash = {
        let cfg = config::config().read().await;
        if let Some(stored_hash) = &cfg.persistent.admin_password_hash {
            let parsed_hash = argon2::PasswordHash::new(stored_hash)
                .map_err(|_| ApiError::Internal("invalid stored password hash".into()))?;
            if argon2::Argon2::default()
                .verify_password(payload.old_password.as_bytes(), &parsed_hash)
                .is_err()
            {
                return Err(ApiError::Unauthorized);
            }
            Some(stored_hash.clone())
        } else {
            None
        }
    };
    // --------------------------------------------------
    // hash the new password outside any lock - argon2 is
    // CPU-bound and we don't want concurrent logins to
    // stall on a hash computation
    // --------------------------------------------------
    use argon2::PasswordHasher;
    let new_hash = argon2::Argon2::default()
        .hash_password(payload.new_password.as_bytes())
        .map_err(|e| ApiError::Internal(format!("failed to hash password: {e}")))?
        .to_string();
    // --------------------------------------------------
    // commit under a write lock with compare-and-swap so
    // a concurrent password change can't be silently
    // clobbered. on mismatch return Unauthorized so the
    // caller refetches and tries again
    // --------------------------------------------------
    {
        let mut cfg = config::config().write().await;
        if cfg.persistent.admin_password_hash != prior_hash {
            return Err(ApiError::Unauthorized);
        }
        cfg.persistent.admin_password_hash = Some(new_hash);
        cfg.persistent
            .save()
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    // --------------------------------------------------
    // invalidate every other session and rotate the caller's
    // own session to a fresh token so they stay logged in
    // --------------------------------------------------
    crate::auth::SESSION_STORE.write().await.clear();
    let new_token = crate::auth::rotate_session(None).await?;
    let cookie = crate::auth::session_cookie_header(&new_token, secure)?;
    // --------------------------------------------------
    // build the response with the rotated session cookie
    // --------------------------------------------------
    let mut response = Json(serde_json::json!({ "success": true })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    tracing::info!(
        "Admin password changed; sessions cleared, caller session rotated (had_prior={})",
        caller_token.is_some()
    );
    Ok(response)
}
