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
use axum::Json;
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
/// new password with argon2, persists the new hash to disk, then invalidates
/// all active sessions so every client must log in again with the new password
/// If no password is currently set the old-password check is skipped entirely
/// Returns 401 Unauthorized if the old password does not match, or 500 if
/// hashing or the disk write fails
///
/// # Arguments
///
/// * `req` - JSON body deserialized into [`ChangePasswordRequest`]; `old_password`
///   must match the currently stored hash (or be ignored when none is set),
///   and `new_password` is the plaintext password to hash and store
pub async fn change_password(
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // --------------------------------------------------
    // acquire a write lock on the config for the duration
    // of the password verification and update
    // --------------------------------------------------
    let mut cfg = config::config().write().await;
    // --------------------------------------------------
    // verify old password if one is currently stored;
    // skip the check entirely when no password is set
    // --------------------------------------------------
    if let Some(stored_hash) = &cfg.persistent.admin_password_hash {
        let parsed_hash = argon2::PasswordHash::new(stored_hash)
            .map_err(|_| ApiError::Internal("invalid stored password hash".into()))?;
        if argon2::Argon2::default()
            .verify_password(req.old_password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(ApiError::Unauthorized);
        }
    }
    // --------------------------------------------------
    // hash the new password with argon2
    // --------------------------------------------------
    use argon2::PasswordHasher;
    let new_hash = argon2::Argon2::default()
        .hash_password(req.new_password.as_bytes())
        .map_err(|e| ApiError::Internal(format!("failed to hash password: {e}")))?
        .to_string();
    // --------------------------------------------------
    // store the new hash and persist to disk
    // --------------------------------------------------
    cfg.persistent.admin_password_hash = Some(new_hash);
    cfg.persistent
        .save()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    drop(cfg);
    // --------------------------------------------------
    // clear all active sessions to force every client to
    // re-authenticate with the new password
    // --------------------------------------------------
    crate::auth::SESSION_STORE.write().await.clear();
    tracing::info!("Admin password changed, sessions cleared");
    Ok(Json(serde_json::json!({ "success": true })))
}
