//! Download link generation and token management
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use crate::core::{
    archive::{DEFAULT_DIR_NAME, DEFAULT_DOWNLOAD_NAME},
    prelude::*,
};
use crate::requests::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

#[get("/api/stats")]
/// Computes stats from the current state
pub async fn stats() -> Result<StatsResponse> {
    let links = crate::APP_STATE.links.read().await;
    let now = std::time::Instant::now();

    let mut active = 0u32;
    let mut used = 0u32;
    let mut expired = 0u32;
    let mut total_downloads = 0u64;
    for item in links.values() {
        let count = item.download_count.load(Ordering::Relaxed);
        total_downloads += count as u64;
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
    Ok(StatsResponse {
        active_links: active,
        used_links: used,
        expired_links: expired,
        total_downloads,
        uptime_seconds: crate::APP_STATE.started_at.elapsed().as_secs(),
    })
}

/// Lists all active download links with their status
#[get("/api/links")]
pub async fn list_links() -> Result<Vec<TokenListItem>> {
    let cfg = CONFIG.read().await;
    let download_base_url = cfg.download_base_url.clone();
    drop(cfg);

    let links = crate::APP_STATE.links.read().await;
    let now = std::time::Instant::now();
    let mut items = Vec::with_capacity(links.len());
    for (token, item) in links.iter() {
        let download_url = super::download_url(&download_base_url, &item.name, token);
        let count = item.download_count.load(Ordering::Relaxed);
        let expired = item.expires_at.map(|e| now >= e).unwrap_or(false);
        let expires_in_seconds = item
            .expires_at
            .filter(|&e| now < e)
            .map(|e| e.duration_since(now).as_secs());
        let archive_status = {
            let state = item.archive_state.read().await;
            match &*state {
                ArchiveState::NotNeeded => "not_needed",
                ArchiveState::Preparing(_) => "preparing",
                ArchiveState::Ready(_) => "ready",
                ArchiveState::Failed(_) => "failed",
            }
        };
        let source_exists = item.paths.iter().all(|p| p.exists());
        items.push(TokenListItem {
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
            archive_status: archive_status.to_string(),
            source_exists,
        });
    }
    Ok(items)
}

#[post("/api/links/generate")]
/// Generates a new download link for the specified files
///
/// # Arguments
///
/// * `req` - The generate request containing paths and options
pub async fn generate_link(req: GenerateRequest) -> Result<GenerateResponse> {
    if req.paths.is_empty() {
        return Err(ServerFnError::new("No paths provided").into());
    }
    let cfg = CONFIG.read().await;
    let base_path = cfg.canonical_base_path.clone();
    let download_base_url = cfg.download_base_url.clone();
    drop(cfg);

    // --------------------------------------------------
    // validate all paths - safe_join canonicalizes and checks containment,
    // blocking both `../` traversal and symlink-based escapes
    // --------------------------------------------------
    let mut full_paths = Vec::new();
    for path_str in &req.paths {
        match super::safe_join(&base_path, path_str) {
            Some(full_path) => full_paths.push(full_path),
            None => {
                tracing::warn!("Generate: path traversal/symlink escape blocked for '{path_str}'");
                return Err(ServerFnError::new("Forbidden").into());
            }
        }
    }
    // --------------------------------------------------
    // build download item
    // --------------------------------------------------
    let token = uuid::Uuid::new_v4().to_string();
    let is_multi_file = full_paths.len() > 1 || (full_paths.len() == 1 && full_paths[0].is_dir());
    let compression = req.format;
    let ext = compression.extension();

    // --------------------------------------------------
    // determine the name
    // --------------------------------------------------
    let name = if let Some(custom_name) = req.name {
        custom_name
    } else if full_paths.len() == 1 {
        let path = &full_paths[0];
        if path.is_dir() {
            format!(
                "{}{}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(DEFAULT_DIR_NAME),
                ext
            )
        } else {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(DEFAULT_DOWNLOAD_NAME)
                .to_string()
        }
    } else {
        format!("output{ext}")
    };

    let max_downloads = req.max_downloads.unwrap_or(1).max(1);
    let expires_at = req
        .expires_in_seconds
        .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs(secs));
    let archive_state = tokio::sync::RwLock::new(if is_multi_file {
        ArchiveState::Preparing(std::time::Instant::now())
    } else {
        ArchiveState::NotNeeded
    });

    // --------------------------------------------------
    // accumulate properties into a DownloadItem
    // --------------------------------------------------
    let item = DownloadItem {
        paths: full_paths.clone(),
        is_multi_file,
        name: name.clone(),
        max_downloads,
        download_count: std::sync::atomic::AtomicU32::new(0),
        expires_at,
        created_at: std::time::Instant::now(),
        compression,
        archive_state,
        active_serving: std::sync::atomic::AtomicU32::new(0),
    };
    crate::APP_STATE
        .links
        .write()
        .await
        .insert(token.clone(), item);
    crate::APP_STATE.dirty.store(true, Ordering::Relaxed);

    // --------------------------------------------------
    // spawn background archive creation for multi-file downloads
    // --------------------------------------------------
    if is_multi_file {
        crate::core::archive::spawn_archive_creation(token.clone(), full_paths, compression);
    }

    // --------------------------------------------------
    // create URL with filename for better wget/browser behavior
    // --------------------------------------------------
    let download_url = super::download_url(&download_base_url, &name, &token);
    tracing::info!("Generated download link for '{name}': {token}");
    // --------------------------------------------------
    // return response
    // --------------------------------------------------
    Ok(GenerateResponse {
        token,
        download_url,
    })
}

/// Deletes a single download link
#[post("/api/links/delete")]
pub async fn delete_link(token: String) -> Result<BulkDeleteResponse> {
    let mut links = crate::APP_STATE.links.write().await;
    let removed = links.remove(&token).inspect(|item| {
        item.remove_cache_file();
        crate::APP_STATE.mark_dirty();
        tracing::info!("Deleted link: {token}");
    });
    let count = removed.is_some() as usize;
    Ok(BulkDeleteResponse { removed: count })
}

#[post("/api/links/revive")]
/// Re-triggers archive creation for a failed or stale multi-file link
pub async fn revive_link(token: String) -> Result<()> {
    let links = crate::APP_STATE.links.read().await;
    // --------------------------------------------------
    // check edge cases
    // --------------------------------------------------
    let Some(item) = links.get(&token) else {
        return Err(ServerFnError::new("Link not found").into());
    };
    if !item.paths.iter().all(|p| p.exists()) {
        return Err(ServerFnError::new("Source files no longer exist").into());
    }
    if !item.is_multi_file && (item.paths.len() != 1 && item.paths[0].is_dir()) {
        return Err(ServerFnError::new("Not an archive link").into());
    }
    // --------------------------------------------------
    // bring the download count back to 0
    // --------------------------------------------------
    item.download_count
        .store(0, std::sync::atomic::Ordering::Relaxed);
    // --------------------------------------------------
    // reset the archive state to Preparing
    // --------------------------------------------------
    let mut archive = item.archive_state.write().await;
    *archive = ArchiveState::Preparing(std::time::Instant::now());
    drop(archive);
    // --------------------------------------------------
    // spawn a new archive creation task
    // --------------------------------------------------
    crate::core::archive::spawn_archive_creation(
        token.clone(),
        item.paths.clone(),
        item.compression,
    );
    // --------------------------------------------------
    // log and return
    // --------------------------------------------------
    tracing::info!("Revive triggered for link {token}");
    Ok(())
}

#[post("/api/links/bulk-delete")]
/// Deletes links matching a filter: "used", "expired", or "all"
pub async fn bulk_delete_links(filter: String) -> Result<BulkDeleteResponse> {
    let mut links = crate::APP_STATE.links.write().await;
    let before = links.len();
    let now = std::time::Instant::now();

    let mut cache_paths: Vec<std::path::PathBuf> = Vec::new();
    // --------------------------------------------------
    // retain only links that don't match the filter
    // --------------------------------------------------
    links.retain(|_, item| {
        let count = item.download_count.load(Ordering::Relaxed);
        let is_expired = item.expires_at.is_some_and(|e| now >= e);
        let is_used = count >= item.max_downloads;
        let keep = match filter.as_str() {
            "used" => !is_used,
            "expired" => !is_expired,
            "all" => false,
            _ => true,
        };
        if !keep
            && item.can_remove_cache()
            && let Some(path) = item.cache_path()
        {
            cache_paths.push(path);
        }
        keep
    });
    let removed = before - links.len();
    if removed > 0 {
        crate::APP_STATE.mark_dirty();
    }
    drop(links);

    // --------------------------------------------------
    // clean up cache files outside the lock
    // --------------------------------------------------
    cache_paths.iter().for_each(super::remove_cache_file);
    // --------------------------------------------------
    // respond with bulk deleted
    // --------------------------------------------------
    tracing::info!("Bulk delete (filter={filter}): removed {removed} links");
    Ok(BulkDeleteResponse { removed })
}
