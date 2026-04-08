//! Background health check and cache cleanup
//!
//! Detects and recovers from:
//! - Archive cache files deleted externally
//! - Archive creation tasks that are stuck in `Preparing` state
//! - Source files deleted while links are still active
//! - Orphaned `.tmp` files from interrupted archive creation
//! - Expired/used links with stale cache files
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::state::ArchiveState;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// How often the health check task runs
const HEALTH_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Maximum time an archive may stay in `Preparing` before being marked `Failed`
const ARCHIVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Minimum age of orphaned `.tmp` files before cleanup (10 minutes)
const TMP_FILE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(600);

/// Runs the health check loop indefinitely
///
/// Must be called **after** [`crate::APP_STATE`] is initialized
pub(crate) async fn health_check_loop() {
    loop {
        // --------------------------------------------------
        // wait for health check interval, then check archives
        // --------------------------------------------------
        tokio::time::sleep(HEALTH_CHECK_INTERVAL).await;
        // --------------------------------------------------
        // single read lock: find stale entries + expired/used
        // archive_state has its own lock so no conflict
        // --------------------------------------------------
        let links = crate::APP_STATE.links.read().await;
        let now = std::time::Instant::now();
        // --------------------------------------------------
        // find archives whose cache file was externally deleted
        // --------------------------------------------------
        let stale: Vec<_> = links
            .iter()
            .filter(|(_, item)| {
                item.is_multi_file || (item.paths.len() == 1 && item.paths[0].is_dir())
            })
            .filter(|(_, item)| {
                item.archive_state
                    .try_read()
                    .ok()
                    .is_some_and(|s| matches!(&*s, ArchiveState::Ready(p) if !p.exists()))
            })
            .map(|(token, _)| token.clone())
            .collect();
        // --------------------------------------------------
        // find archives stuck in Preparing past the timeout
        // --------------------------------------------------
        let stuck: Vec<_> = links
            .iter()
            .filter(|(_, item)| {
                item.archive_state.try_read().ok().is_some_and(|s| {
                    matches!(&*s, ArchiveState::Preparing(since) if now.duration_since(*since) >= ARCHIVE_TIMEOUT)
                })
            })
            .map(|(token, _)| token.clone())
            .collect();
        // --------------------------------------------------
        // find expired/used links that still have a cache file
        // --------------------------------------------------
        let to_clean: Vec<std::path::PathBuf> = links
            .values()
            .filter(|item| {
                let count = item
                    .download_count
                    .load(std::sync::atomic::Ordering::Relaxed);
                let is_expired = item.expires_at.is_some_and(|e| now >= e);
                let is_used = count >= item.max_downloads;
                (is_expired || is_used) && item.can_remove_cache()
            })
            .filter_map(|item| item.cache_path())
            .filter(|p| p.exists())
            .collect();
        // --------------------------------------------------
        // warn about active links whose source files are gone
        // --------------------------------------------------
        for (token, item) in links.iter() {
            let count = item
                .download_count
                .load(std::sync::atomic::Ordering::Relaxed);
            let is_active = item.expires_at.is_none_or(|e| now < e) && count < item.max_downloads;
            if is_active && !item.paths.iter().all(|p| p.exists()) {
                tracing::warn!("Source files missing for active link {token}");
            }
        }
        // --------------------------------------------------
        // write-lock each stale archive individually
        // re-check because state may have changed since collect
        // --------------------------------------------------
        for token in stale {
            if let Some(item) = links.get(&token) {
                let mut archive = item.archive_state.write().await;
                if matches!(&*archive, ArchiveState::Ready(p) if !p.exists()) {
                    *archive = ArchiveState::Failed("Archive cache deleted".into());
                    tracing::info!("Marked stale archive for token {token}");
                }
            }
        }
        // --------------------------------------------------
        // mark stuck Preparing archives as Failed
        // --------------------------------------------------
        for token in stuck {
            if let Some(item) = links.get(&token) {
                let mut archive = item.archive_state.write().await;
                if matches!(&*archive, ArchiveState::Preparing(since) if now.duration_since(*since) >= ARCHIVE_TIMEOUT)
                {
                    *archive = ArchiveState::Failed("Archive creation timed out".into());
                    tracing::warn!("Archive creation timed out for token {token}");
                }
            }
        }
        drop(links);
        // --------------------------------------------------
        // clean up expired/used cache files
        // --------------------------------------------------
        to_clean.iter().for_each(super::remove_cache_file);
        // --------------------------------------------------
        // clean up orphaned .tmp files in the archive cache dir
        // --------------------------------------------------
        cleanup_orphaned_tmp_files();
    }
}

/// Removes `.tmp` files in the archive cache directory that are older than
/// [`TMP_FILE_MAX_AGE`], indicating interrupted archive creation
fn cleanup_orphaned_tmp_files() {
    let Ok(entries) = std::fs::read_dir(crate::core::archive::ARCHIVE_CACHE_DIR) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "tmp") {
            let is_stale = path
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age >= TMP_FILE_MAX_AGE);
            if is_stale {
                let _ = std::fs::remove_file(&path);
                tracing::info!("Removed orphaned tmp file: {path:?}");
            }
        }
    }
}
