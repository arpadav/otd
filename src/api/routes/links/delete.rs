//! Delete a single download link by token
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{Json, extract::Path};

/// Handles `DELETE /api/links/:token`
///
/// Acquires a write lock on the link map, removes the entry for the given
/// token (if it exists), and on removal cleans up any associated archive
/// cache file, marks state dirty, and logs the deletion. Returns `removed: 1`
/// if the link existed or `removed: 0` if no such token was found
///
/// # Arguments
///
/// * `token` - Path parameter containing the token of the link to delete
pub async fn delete_link(
    Path(token): Path<String>,
) -> Result<Json<super::BulkDeleteResponse>, ApiError> {
    // --------------------------------------------------
    // acquire write lock on the link map
    // --------------------------------------------------
    let mut links = crate::APP_STATE.links.write().await;
    // --------------------------------------------------
    // remove the entry and clean up its cache file if found
    // --------------------------------------------------
    let removed = links.remove(&token).inspect(|item| {
        item.remove_cache_file();
        crate::APP_STATE.mark_dirty();
        tracing::info!("Deleted link: {token}");
    });
    // --------------------------------------------------
    // return removed count (1 if found, 0 if not found)
    // --------------------------------------------------
    let count = removed.is_some() as usize;
    Ok(Json(super::BulkDeleteResponse { removed: count }))
}
