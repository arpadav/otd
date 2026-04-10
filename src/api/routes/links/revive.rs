//! Revive a download link by resetting its state and re-triggering archive creation
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::state::ArchiveState;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{Json, extract::Path};
use std::sync::atomic::Ordering;

/// Handles `POST /api/links/:token/revive`
///
/// Resets the download count to 0 and re-triggers archive creation for
/// multi-file or directory links. Returns an error if the link is missing,
/// if any of its source files no longer exist on disk, or if it is a plain
/// single-file link that does not require an archive
///
/// # Arguments
///
/// * `token` - Path parameter identifying the link to revive
pub async fn revive_link(Path(token): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    // --------------------------------------------------
    // acquire read lock on the link map
    // --------------------------------------------------
    let links = crate::APP_STATE.links.read().await;
    // --------------------------------------------------
    // return 404 if the token does not exist
    // --------------------------------------------------
    let Some(item) = links.get(&token) else {
        return Err(ApiError::NotFound("Link not found".into()));
    };
    // --------------------------------------------------
    // reject if any source file no longer exists on disk
    // --------------------------------------------------
    if !item.paths.iter().all(|p| p.exists()) {
        return Err(ApiError::BadRequest("Source files no longer exist".into()));
    }
    // --------------------------------------------------
    // reject single-file links that are not directories
    // (only archive links can be revived)
    // --------------------------------------------------
    if !item.is_multi_file && (item.paths.len() != 1 || !item.paths[0].is_dir()) {
        return Err(ApiError::BadRequest("Not an archive link".into()));
    }
    // --------------------------------------------------
    // reset the download count to zero
    // --------------------------------------------------
    item.download_count.store(0, Ordering::Relaxed);
    // --------------------------------------------------
    // reset the archive state to Preparing
    // --------------------------------------------------
    let mut archive = item.archive_state.write().await;
    *archive = ArchiveState::Preparing(std::time::Instant::now());
    drop(archive);
    // --------------------------------------------------
    // spawn a new archive creation task
    // --------------------------------------------------
    crate::archive::spawn_archive_creation(token.clone(), item.paths.clone(), item.compression);
    // --------------------------------------------------
    // log the revive and return the token
    // --------------------------------------------------
    tracing::info!("Revive triggered for link {token}");
    Ok(Json(serde_json::json!({ "token": token })))
}
