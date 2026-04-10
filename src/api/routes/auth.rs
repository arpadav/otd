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
