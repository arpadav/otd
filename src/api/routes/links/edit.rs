//! Link editing handler
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
use axum::Json;
use axum::extract::Path;
use serde::Deserialize;
use std::sync::atomic::Ordering;
use std::time::Instant;

// --------------------------------------------------
// types
// --------------------------------------------------
#[derive(Debug, Deserialize)]
/// Request payload for `PUT /api/links/{token}`
pub(crate) struct UpdateLinkRequest {
    /// New maximum number of downloads allowed
    pub max_downloads: u32,
    /// New expiry in seconds from now (`None` = no expiry)
    pub expires_in_seconds: Option<u64>,
}

/// Handles `PUT /api/links/{token}`
///
/// Updates the `max_downloads` and `expires_at` fields of an existing link
/// without touching its paths, name, or archive state. Marks state dirty so
/// the change is persisted to disk on the next flush
///
/// # Arguments
///
/// * `token` - Path parameter identifying the link to edit
/// * `req` - JSON body deserialized into [`UpdateLinkRequest`]
pub async fn edit_link(
    Path(token): Path<String>,
    Json(req): Json<UpdateLinkRequest>,
) -> Result<Json<super::TokenListItem>, ApiError> {
    // --------------------------------------------------
    // validate max_downloads is at least 1
    // --------------------------------------------------
    let max_downloads = req.max_downloads.max(1);
    // --------------------------------------------------
    // compute new expiry instant from seconds offset
    // --------------------------------------------------
    let expires_at = req
        .expires_in_seconds
        .map(|secs| Instant::now() + std::time::Duration::from_secs(secs));
    // --------------------------------------------------
    // acquire write lock on link map and find the entry
    // --------------------------------------------------
    let mut links = crate::APP_STATE.links.write().await;
    let item = links
        .get_mut(&token)
        .ok_or_else(|| ApiError::NotFound(format!("Link '{token}' not found")))?;
    // --------------------------------------------------
    // apply updates
    // --------------------------------------------------
    item.max_downloads = max_downloads;
    item.expires_at = expires_at;
    // --------------------------------------------------
    // mark state dirty for persistence
    // --------------------------------------------------
    crate::APP_STATE.dirty.store(true, Ordering::Relaxed);
    // --------------------------------------------------
    // build response from updated item
    // --------------------------------------------------
    let cfg = config::config().read().await;
    let now = Instant::now();
    let download_url = cfg.download_url(&item.name, &token);
    let count = item.download_count.load(Ordering::Relaxed);
    let expired = item.expires_at.map(|e| now >= e).unwrap_or(false);
    let expires_in_seconds = item
        .expires_at
        .filter(|&e| now < e)
        .map(|e| e.duration_since(now).as_secs());
    let link_status = if expired {
        crate::shared::LinkStatuses::Expired
    } else {
        let state = item.archive_state.read().await;
        crate::shared::LinkStatuses::from(&*state)
    };
    let source_exists = item.paths.iter().all(|p| p.exists());
    let response = super::TokenListItem {
        token: token.clone(),
        name: item.name.clone(),
        is_multi_file: item.is_multi_file,
        download_count: count,
        max_downloads,
        remaining_downloads: max_downloads.saturating_sub(count),
        expired,
        expires_in_seconds,
        download_url,
        paths: item
            .paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        link_status,
        source_exists,
    };
    tracing::info!("Updated link '{token}': max_downloads={max_downloads}");
    Ok(Json(response))
}
