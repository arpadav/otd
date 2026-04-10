//! Download link generation handler
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::archive::CompressionType;
use crate::archive::{DEFAULT_DIR_NAME, DEFAULT_DOWNLOAD_NAME};
use crate::config;
use crate::state::{ArchiveState, DownloadItem};

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::Json;
use std::sync::atomic::Ordering;

/// Handles `POST /api/links/generate`
///
/// Validates that at least one path was provided, then resolves each path
/// against the configured base directory using `safe_join` to block directory
/// traversal and symlink escapes. Generates a UUID token, determines the
/// download name (custom, inferred from the single path, or a default),
/// builds a [`DownloadItem`], inserts it into [`crate::APP_STATE`], marks
/// state dirty, and optionally spawns a background archive task for
/// multi-file or directory downloads. Returns the token and its download URL
///
/// # Arguments
///
/// * `req` - JSON body deserialised into a [`super::GenerateRequest`]
pub async fn generate_link(
    Json(req): Json<super::GenerateRequest>,
) -> Result<Json<super::GenerateResponse>, ApiError> {
    // --------------------------------------------------
    // reject requests with no paths
    // --------------------------------------------------
    if req.paths.is_empty() {
        return Err(ApiError::BadRequest("No paths provided".into()));
    }
    // --------------------------------------------------
    // acquire read lock on config
    // --------------------------------------------------
    let cfg = config::config().read().await;
    // --------------------------------------------------
    // resolve and validate each path via safe_join, which
    // canonicalizes and checks base-dir containment to
    // block directory traversal and symlink escapes
    // --------------------------------------------------
    let mut full_paths = Vec::with_capacity(req.paths.len());
    for path_str in &req.paths {
        match cfg.safe_join(path_str) {
            Some(full_path) => full_paths.push(full_path),
            None => {
                tracing::warn!("Generate: path traversal/symlink escape blocked for '{path_str}'");
                return Err(ApiError::Forbidden("Forbidden".into()));
            }
        }
    }
    // --------------------------------------------------
    // generate a unique token for this download link
    // --------------------------------------------------
    let token = uuid::Uuid::new_v4().to_string();
    // --------------------------------------------------
    // determine whether this is a multi-file or directory
    // download, and select the compression type
    // --------------------------------------------------
    let is_multi_file = full_paths.len() > 1 || (full_paths.len() == 1 && full_paths[0].is_dir());
    let compression: CompressionType = req.format;
    let ext = compression.extension();
    // --------------------------------------------------
    // determine the download name: use the custom name if
    // provided, infer from the single path, or fall back
    // to a default
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
    // --------------------------------------------------
    // resolve download limit and optional expiry instant
    // --------------------------------------------------
    let max_downloads = req.max_downloads.unwrap_or(1).max(1);
    let expires_at = req
        .expires_in_seconds
        .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs(secs));
    // --------------------------------------------------
    // set initial archive state: Preparing for multi-file
    // downloads, NotNeeded for single-file downloads
    // --------------------------------------------------
    let archive_state = tokio::sync::RwLock::new(if is_multi_file {
        ArchiveState::Preparing(std::time::Instant::now())
    } else {
        ArchiveState::NotNeeded
    });
    // --------------------------------------------------
    // build the DownloadItem from all resolved fields
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
    // --------------------------------------------------
    // insert the item into app state and mark it dirty
    // --------------------------------------------------
    crate::APP_STATE
        .links
        .write()
        .await
        .insert(token.clone(), item);
    crate::APP_STATE.dirty.store(true, Ordering::Relaxed);
    // --------------------------------------------------
    // spawn background archive creation for multi-file
    // or directory downloads
    // --------------------------------------------------
    if is_multi_file {
        crate::archive::spawn_archive_creation(token.clone(), full_paths, compression);
    }
    // --------------------------------------------------
    // build the download URL, release config lock, and log
    // --------------------------------------------------
    let download_url = cfg.download_url(&name, &token);
    drop(cfg);
    tracing::info!("Generated download link for '{name}': {token}");
    // --------------------------------------------------
    // return the token and its download URL
    // --------------------------------------------------
    Ok(Json(super::GenerateResponse {
        token,
        download_url,
    }))
}
