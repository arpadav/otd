//! Link statistics handler
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::Json;
use std::sync::atomic::Ordering;

/// Handles `GET /api/links/stats`
///
/// Acquires a read lock on the link map and iterates over all entries,
/// tallying active, used, and expired links and summing total downloads
/// A link is expired if its `expires_at` instant has passed, used if its
/// download count has reached `max_downloads`, and active otherwise
/// Returns the computed counts alongside the server uptime in seconds
pub async fn stats() -> Json<super::StatsResponse> {
    // --------------------------------------------------
    // acquire read lock on the link map
    // --------------------------------------------------
    let links = crate::APP_STATE.links.read().await;
    // --------------------------------------------------
    // snapshot the current instant for expiry comparisons
    // --------------------------------------------------
    let now = std::time::Instant::now();
    // --------------------------------------------------
    // accumulate per-link counters
    // --------------------------------------------------
    let mut active = 0u32;
    let mut used = 0u32;
    let mut expired = 0u32;
    let mut total_downloads = 0u64;
    for item in links.values() {
        let count = item.download_count.load(Ordering::Relaxed);
        total_downloads += u64::from(count);
        let is_expired = item.expires_at.is_some_and(|e| now >= e);
        let is_used = count >= item.max_downloads;
        if is_expired {
            expired += 1;
        } else if is_used {
            used += 1;
        } else {
            active += 1;
        }
    }
    // --------------------------------------------------
    // build and return the stats response
    // --------------------------------------------------
    Json(super::StatsResponse {
        active_links: active,
        used_links: used,
        expired_links: expired,
        total_downloads,
        uptime_seconds: crate::APP_STATE.started_at.elapsed().as_secs(),
    })
}
