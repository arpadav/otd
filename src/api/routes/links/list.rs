//! Link listing handler
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::config;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::Json;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Handles `GET /api/links`
///
/// Acquires read locks on the config and link map, then iterates all entries
/// to build a list of [`super::TokenListItem`] values enriched with computed
/// fields: remaining downloads, expiry status, archive state, and whether all
/// source paths still exist on disk. The resulting list is sorted by creation
/// time, newest first, before the locks are released
pub async fn list_links() -> Json<Vec<super::TokenListItem>> {
    // --------------------------------------------------
    // acquire read locks on config and link map
    // --------------------------------------------------
    let cfg = config::config().read().await;
    let links = crate::APP_STATE.links.read().await;
    // --------------------------------------------------
    // snapshot the current instant for expiry comparisons
    // --------------------------------------------------
    let now = Instant::now();
    // --------------------------------------------------
    // allocate output vec carrying creation time so we
    // can sort by it without re-locking
    // --------------------------------------------------
    let mut items: Vec<(Instant, super::TokenListItem)> = Vec::with_capacity(links.len());
    // --------------------------------------------------
    // build a TokenListItem for each link in the map
    // --------------------------------------------------
    for (token, item) in links.iter() {
        // --------------------------------------------------
        // compute per-link derived fields: download URL,
        // count, expiry status, and remaining seconds
        // --------------------------------------------------
        let download_url = cfg.download_url(&item.name, token);
        let count = item.download_count.load(Ordering::Relaxed);
        let expired = item.expires_at.map(|e| now >= e).unwrap_or(false);
        let expires_in_seconds = item
            .expires_at
            .filter(|&e| now < e)
            .map(|e| e.duration_since(now).as_secs());
        // --------------------------------------------------
        // read archive state and map it to a status string
        // --------------------------------------------------
        let link_status = if expired {
            crate::shared::LinkStatuses::Expired
        } else {
            let state = item.archive_state.read().await;
            crate::shared::LinkStatuses::from(&*state)
        };
        // --------------------------------------------------
        // check whether all source paths still exist on disk
        // --------------------------------------------------
        let source_exists = item.paths.iter().all(|p| p.exists());
        // --------------------------------------------------
        // push the enriched item alongside its creation time
        // --------------------------------------------------
        items.push((
            item.created_at,
            super::TokenListItem {
                token: token.clone(),
                name: item.name.clone(),
                is_multi_file: item.is_multi_file,
                download_count: count,
                max_downloads: item.max_downloads,
                remaining_downloads: item.max_downloads.saturating_sub(count),
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
            },
        ));
    }
    // --------------------------------------------------
    // release read locks before sorting
    // --------------------------------------------------
    drop(links);
    drop(cfg);
    // --------------------------------------------------
    // sort by creation time, newest first
    // --------------------------------------------------
    items.sort_by(|(ta, _), (tb, _)| tb.cmp(ta));
    // --------------------------------------------------
    // strip the sort key and return the ordered list
    // --------------------------------------------------
    Json(items.into_iter().map(|(_, item)| item).collect())
}
