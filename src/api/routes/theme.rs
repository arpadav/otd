//! Theme preference endpoint and types
//!
//! Persists the active theme to `$XDG_DATA_HOME/otd/theme.json`
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::shared::{ThemeMode, ThemeName};

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::Json;
use serde::{Deserialize, Serialize};

// --------------------------------------------------
// types
// --------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Persisted theme preference
///
/// Serialized to `$XDG_DATA_HOME/otd/theme.json` by [`set_theme`] and read
/// back by [`get_theme`]. Falls back to [`Default`] when the file is absent
/// or unparseable
pub struct ThemePreference {
    /// Theme name (e.g. "forest", "clay", "ocean")
    pub name: String,
    /// Color mode: "light" or "dark"
    pub mode: String,
}

/// [`ThemePreference`] implementation of [`Default`]
impl Default for ThemePreference {
    fn default() -> Self {
        // --------------------------------------------------
        // use the first entry from each allowed-values list
        // as the canonical default theme and mode
        // --------------------------------------------------
        let default_mode = ThemeMode::STRS[0];
        let default_name = ThemeName::STRS[0];
        Self {
            name: default_name.to_string(),
            mode: default_mode.to_string(),
        }
    }
}

#[inline(always)]
/// Returns the path to the theme preference file
///
/// Constructs the path as `$XDG_DATA_HOME/otd/theme.json` using the
/// application data directory resolved at runtime
fn theme_path() -> std::path::PathBuf {
    crate::config::data_dir().join("theme.json")
}

/// Handles `GET /api/theme`
///
/// Reads the persisted theme preference from disk and returns it as JSON
///
/// If the file does not exist or cannot be deserialized, returns
/// [`ThemePreference::default`] silently without an error
pub async fn get_theme() -> Json<ThemePreference> {
    // --------------------------------------------------
    // resolve the theme file path
    // --------------------------------------------------
    let path = theme_path();
    // --------------------------------------------------
    // attempt to read and deserialize; fall back to default
    // if the file is absent or malformed
    // --------------------------------------------------
    let pref = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<ThemePreference>(&s).ok())
        .unwrap_or_default();
    Json(pref)
}

/// Handles `PUT /api/theme`
///
/// Validates that both the theme name and color mode are recognized values,
/// then serializes the preference to `$XDG_DATA_HOME/otd/theme.json`,
/// creating parent directories as needed. Returns the saved preference on
/// success, or a 400 Bad Request if either field is unrecognized
///
/// # Arguments
///
/// * `pref` - JSON body deserialized into [`ThemePreference`]; both `name`
///   and `mode` must match entries in [`ThemeName::STRS`] and
///   [`ThemeMode::STRS`] respectively
pub async fn set_theme(
    Json(pref): Json<ThemePreference>,
) -> Result<Json<ThemePreference>, ApiError> {
    // --------------------------------------------------
    // validate that the theme name is a recognized value
    // --------------------------------------------------
    if ThemeName::parse(&pref.name).is_none() {
        return Err(ApiError::BadRequest(format!(
            "Unknown theme '{}'. Valid: {}",
            pref.name,
            ThemeName::STRS.join(", ")
        )));
    }
    // --------------------------------------------------
    // validate that the color mode is a recognized value
    // --------------------------------------------------
    if ThemeMode::parse(&pref.mode).is_none() {
        return Err(ApiError::BadRequest(format!(
            "Unknown mode '{}'. Valid: {}",
            pref.mode,
            ThemeMode::STRS.join(", ")
        )));
    }
    // --------------------------------------------------
    // ensure the data directory exists, then write the
    // serialized preference to the theme file
    // --------------------------------------------------
    let path = theme_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::Internal(format!("Failed to create data directory: {e}")))?;
    }
    let json = serde_json::to_string_pretty(&pref)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize theme: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| ApiError::Internal(format!("Failed to write theme file: {e}")))?;
    tracing::info!("Theme updated: {} ({})", pref.name, pref.mode);
    Ok(Json(pref))
}
