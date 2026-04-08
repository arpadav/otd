//! Admin authentication
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use super::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

/// Validates the admin password
///
/// Returns `true` if the password matches the configured admin password,
/// or if no password is configured (open access)
///
/// # Arguments
///
/// * `password` - The password to validate
#[post("/api/login")]
pub async fn login(password: String) -> Result<bool> {
    let cfg = CONFIG.read().await;
    match &cfg.raw.admin_password {
        Some(expected) if expected == &password => {
            // TODO: create session cookie
            Ok(true)
        }
        Some(_) => Ok(false),
        None => {
            // No password configured, login always succeeds
            Ok(true)
        }
    }
}
