//! Authentication request and response types
//!
//! Handlers live in [`crate::auth`] alongside the middleware
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
/// Login request with password
pub struct LoginRequest {
    /// Admin password
    pub password: String,
}

#[derive(Debug, Serialize)]
/// Login response indicating success or failure
pub struct LoginResponse {
    /// Whether authentication was successful
    pub success: bool,
}

#[derive(Debug, Serialize)]
/// Response payload for `GET /api/auth/me`
///
/// Reports whether the request carries a valid session and whether the
/// server has a password configured at all. The frontend uses this on
/// mount to decide whether to render the authenticated shell or redirect
/// to the login page
pub struct MeResponse {
    /// Whether the request carries a valid session (or no password is set)
    pub logged_in: bool,
    /// Whether an admin password is configured on the server
    pub password_required: bool,
}
