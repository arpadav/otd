//! Bulk-delete download links by filter
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::shared::BulkDeleteFilters;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{Json, extract::Query};
use serde::Deserialize;
use std::sync::atomic::Ordering;

#[derive(Deserialize)]
/// Query parameters for the bulk-delete endpoint
pub(crate) struct BulkDeleteQuery {
    #[serde(default)]
    /// Filter controlling which links are removed: `"used"`, `"expired"`, or `"all"`
    filter: String,
}

/// Handles `DELETE /api/links`
///
/// Parses the `filter` query parameter against the [`BulkDeleteFilters`] enum
/// and rejects unknown values with a 400 Bad Request. Acquires a write lock
/// on the link map, retains only links that do not match the filter, and
/// removes the archive cache file for each discarded link. Marks state dirty
/// if any links were removed. Valid filter values are:
///
/// - `"used"` - links where `download_count >= max_downloads`
/// - `"expired"` - links where `expires_at` is in the past
/// - `"all"` - every link regardless of state
///
/// # Arguments
///
/// * `params` - Query string deserialised into [`BulkDeleteQuery`]
pub async fn bulk_delete_links(
    Query(params): Query<BulkDeleteQuery>,
) -> Result<Json<super::BulkDeleteResponse>, ApiError> {
    // --------------------------------------------------
    // validate the filter string against the shared enum
    // --------------------------------------------------
    let filter = BulkDeleteFilters::parse(&params.filter).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "Unknown filter '{}'; expected one of: {}",
            params.filter,
            BulkDeleteFilters::STRS.join(", "),
        ))
    })?;
    // --------------------------------------------------
    // acquire write lock on the link map
    // --------------------------------------------------
    let mut links = crate::APP_STATE.links.write().await;
    // --------------------------------------------------
    // snapshot pre-removal count and current instant for
    // expiry comparisons
    // --------------------------------------------------
    let before = links.len();
    let now = std::time::Instant::now();
    // --------------------------------------------------
    // retain only links that don't match the filter;
    // remove the cache file for each discarded entry
    // --------------------------------------------------
    links.retain(|_, item| {
        let count = item.download_count.load(Ordering::Relaxed);
        let is_expired = item.expires_at.is_some_and(|e| now >= e);
        let is_used = count >= item.max_downloads;
        let keep = match filter {
            BulkDeleteFilters::Used => !is_used,
            BulkDeleteFilters::Expired => !is_expired,
            BulkDeleteFilters::All => false,
        };
        if !keep {
            item.remove_cache_file();
        }
        keep
    });
    // --------------------------------------------------
    // compute removed count and mark state dirty if any
    // links were removed
    // --------------------------------------------------
    let removed = before - links.len();
    if removed > 0 {
        crate::APP_STATE.mark_dirty();
    }
    // --------------------------------------------------
    // release write lock before logging and responding
    // --------------------------------------------------
    drop(links);
    // --------------------------------------------------
    // log the result and return the removed count
    // --------------------------------------------------
    tracing::info!(
        "Bulk delete (filter={}): removed {removed} links",
        filter.as_str()
    );
    Ok(Json(super::BulkDeleteResponse { removed }))
}
