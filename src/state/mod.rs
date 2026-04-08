//! Core runtime state for the OTD server
//!
//! This module defines the server-side runtime data structures:
//! [`DownloadItem`], [`ArchiveState`], and [`AppState`]
//!
//! Request/response DTO types have moved to [`crate::requests`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    core::archive::{ARCHIVE_CACHE_DIR, CompressionType},
    requests::PersistedDownloadItem,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU32, Ordering},
    },
};

// --------------------------------------------------
// external
// --------------------------------------------------
use tokio::sync::RwLock;

// --------------------------------------------------
// constants
// --------------------------------------------------
#[cfg(feature = "server")]
/// Per-link storage subdirectory within the data directory
const LINKS_DIR: &str = "links";

#[cfg(feature = "server")]
#[derive(Debug, Clone)]
/// Tracks the lifecycle of an archive for a download item
pub(crate) enum ArchiveState {
    /// Single-file download - no archive needed
    NotNeeded,
    /// Archive is being created in the background (tracks when preparation started)
    Preparing(std::time::Instant),
    /// Archive is ready and cached at the given path
    Ready(PathBuf),
    /// Archive creation failed
    Failed(String),
}

/// Global app state, set once during server init
pub(crate) static APP_STATE: LazyLock<Arc<AppState>> = LazyLock::new(|| {
    tracing::info!("starting init app state");
    std::fs::create_dir_all(ARCHIVE_CACHE_DIR).ok();
    let data_dir = crate::config::data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    // --------------------------------------------------
    // load persisted state or start fresh
    // --------------------------------------------------
    let state = match AppState::load_state(&data_dir) {
        Ok(links) => {
            tracing::info!("Loaded {} persisted links from {data_dir:?}", links.len());
            Arc::new(AppState::with_links(links))
        }
        Err(_) => {
            tracing::debug!(
                "No persisted state loaded, either corrupt or first run. Starting fresh."
            );
            Arc::new(AppState::new())
        }
    };
    tracing::info!("finish init app state");
    state
});

/// Spawns background tasks that depend on [`APP_STATE`]
///
/// Must be called **after** `LazyLock::force(&APP_STATE)` to avoid
/// deadlock -- the tasks reference the global directly
pub(crate) fn spawn_background_tasks() {
    // --------------------------------------------------
    // re-trigger archive creation for active multi-file links
    // --------------------------------------------------
    tokio::spawn(async {
        let links_read = crate::APP_STATE.links.read().await;
        links_read
            .iter()
            .filter(|(_, item)| {
                item.is_multi_file
                    && item.download_count.load(Ordering::Relaxed) < item.max_downloads
                    && item
                        .expires_at
                        .is_none_or(|e| std::time::Instant::now() < e)
            })
            .for_each(|(token, item)| {
                crate::core::archive::spawn_archive_creation(
                    token.clone(),
                    item.paths.clone(),
                    item.compression,
                );
            });
    });
    // --------------------------------------------------
    // state persistence loop - saves dirty state every second
    // --------------------------------------------------
    tokio::spawn(async {
        let data_dir = crate::config::data_dir();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if crate::APP_STATE
                .dirty
                .swap(false, std::sync::atomic::Ordering::AcqRel)
            {
                if let Err(e) = crate::APP_STATE.save_state(&data_dir).await {
                    tracing::warn!("Failed to persist state: {e}");
                } else {
                    tracing::debug!("State persisted to {data_dir:?}");
                }
            }
        }
    });
    // --------------------------------------------------
    // health check loop - cleans stale/expired archives
    // --------------------------------------------------
    tokio::spawn(async {
        crate::core::health::health_check_loop().await;
    });
}

#[cfg(feature = "server")]
#[derive(Debug)]
/// Represents a downloadable item with one or more files/folders
///
/// Each download item is associated with a unique token and can contain
/// multiple paths that will be served as a single download (zip for multiple items)
pub(crate) struct DownloadItem {
    /// List of file/folder paths included in this download
    pub(crate) paths: Vec<PathBuf>,

    /// Whether this download contains multiple files/folders (true if paths.len() > 1)
    pub(crate) is_multi_file: bool,

    /// Display name for the download
    pub(crate) name: String,

    /// Maximum allowed downloads before the link becomes invalid
    pub(crate) max_downloads: u32,

    /// Current download count for this item
    pub(crate) download_count: AtomicU32,

    /// Optional expiration time for the download link (None if it does not expire)
    pub(crate) expires_at: Option<std::time::Instant>,

    /// When this download item was created
    pub(crate) created_at: std::time::Instant,

    /// Compression format for archive downloads
    pub(crate) compression: CompressionType,

    /// Archive preparation state (interior-mutable; never held across .await)
    pub(crate) archive_state: tokio::sync::RwLock<ArchiveState>,

    /// Number of downloads currently being served (file read in progress)
    /// Cache files must not be deleted while this is > 0
    pub(crate) active_serving: AtomicU32,
}

#[cfg(feature = "server")]
/// [`DownloadItem`] implementation
impl DownloadItem {
    #[inline(always)]
    /// Returns the archive cache path if the archive state is [`ArchiveState::Ready`]
    ///
    /// Uses a non-blocking `try_read` so it never stalls callers
    pub(crate) fn cache_path(&self) -> Option<PathBuf> {
        self.archive_state.try_read().ok().and_then(|s| match &*s {
            ArchiveState::Ready(p) => Some(p.clone()),
            _ => None,
        })
    }

    #[inline(always)]
    /// Whether it is safe to remove this item's cache file right now
    ///
    /// Returns `false` if any download is actively being served (file read
    /// in progress), since deleting the cache mid-read would cause the
    /// download to fail
    pub(crate) fn can_remove_cache(&self) -> bool {
        self.active_serving
            .load(std::sync::atomic::Ordering::Acquire)
            == 0
    }

    /// Removes the archive cache file from disk if no download is in progress
    ///
    /// # Returns
    ///
    /// * `true` if the file was removed or didn't exist
    /// * `false` if skipped because a download is in progress
    pub(crate) fn remove_cache_file(&self) -> bool {
        if !self.can_remove_cache() {
            tracing::debug!("Skipping cache removal: download in progress");
            return false;
        }
        self.cache_path()
            .as_ref()
            .map(crate::core::remove_cache_file);
        true
    }
}

#[cfg(feature = "server")]
/// [`DownloadItem`] implementation of [`From`] for [`PersistedDownloadItem`]
impl From<PersistedDownloadItem> for DownloadItem {
    fn from(item: PersistedDownloadItem) -> Self {
        let now = std::time::Instant::now();
        let expires_at = item
            .expires_in_seconds
            .map(|secs| now + std::time::Duration::from_secs(secs));
        let created_at = now - std::time::Duration::from_secs(item.created_ago_seconds);
        let is_used = item.download_count >= item.max_downloads;
        let is_expired = item.expires_in_seconds == Some(0);
        let archive_state = if !item.is_multi_file || is_used || is_expired {
            // --------------------------------------------------
            // single-file / used / expired - no achive needed
            // --------------------------------------------------
            tokio::sync::RwLock::new(ArchiveState::NotNeeded)
        } else {
            // --------------------------------------------------
            // active multi-file - will be updated after checking
            // cache or re-creating
            // --------------------------------------------------
            tokio::sync::RwLock::new(ArchiveState::Preparing(std::time::Instant::now()))
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        DownloadItem {
            paths: item.paths,
            is_multi_file: item.is_multi_file,
            name: item.name,
            max_downloads: item.max_downloads,
            download_count: AtomicU32::new(item.download_count),
            expires_at,
            created_at,
            compression: item.compression,
            archive_state,
            active_serving: AtomicU32::new(0),
        }
    }
}

#[cfg(feature = "server")]
/// Shared application state containing configuration and active downloads
///
/// This structure is shared between all request handlers and contains the
/// core application data including active download links and configuration
///
/// # Examples
///
/// ```rust
/// use otd::types::AppState;
///
/// let state = AppState::new();
/// ```
pub struct AppState {
    /// Map of active download tokens to their corresponding items
    pub(crate) links: RwLock<HashMap<String, DownloadItem>>,
    /// Server start time for uptime tracking
    pub(crate) started_at: std::time::Instant,
    /// Set when links change; cleared after state is persisted
    pub(crate) dirty: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "server")]
/// [`AppState`] implementation of [`Default`]
impl Default for AppState {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "server")]
/// [`AppState`] implementation
impl AppState {
    #[inline(always)]
    /// Creates a new application state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::types::AppState;
    ///
    /// let state = AppState::new();
    /// ```
    pub fn new() -> Self {
        Self {
            links: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Creates an `AppState` pre-loaded with persisted links
    pub(crate) fn with_links(links: HashMap<String, DownloadItem>) -> Self {
        Self {
            links: RwLock::new(links),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Marks the state as dirty so it will be persisted on the next save cycle
    pub(crate) fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::Release);
    }

    /// Saves the current token state to disk as individual per-link JSON files
    ///
    /// Each token is written as `links/<token>.json` via atomic `.tmp` rename
    /// After writing, orphan files (links no longer in memory) are removed
    pub(crate) async fn save_state(
        &self,
        dir: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let links_dir = dir.join(LINKS_DIR);
        // --------------------------------------------------
        // re-create links dir if deleted while running - memory
        // is authoritative!
        // --------------------------------------------------
        std::fs::create_dir_all(&links_dir)?;
        let now = std::time::Instant::now();
        let links = self.links.read().await;
        // --------------------------------------------------
        // write each token as an individual file
        // --------------------------------------------------
        let mut live_stems: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(links.len());
        for (token, item) in links.iter() {
            live_stems.insert(token.clone());
            let persisted = PersistedDownloadItem::from_download_item(item, now);
            let json = serde_json::to_string_pretty(&persisted)?;
            let file_path = links_dir.join(format!("{token}.json"));
            let tmp_path = links_dir.join(format!("{token}.json.tmp"));
            std::fs::write(&tmp_path, json)?;
            std::fs::rename(&tmp_path, &file_path)?;
        }
        drop(links);
        // --------------------------------------------------
        // remove orphan files whose stem isn't in the
        // in-memory token set
        // --------------------------------------------------
        if let Ok(entries) = std::fs::read_dir(&links_dir) {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()) == Some("json")
                        && p.file_stem()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| !live_stems.contains(s))
                })
                .for_each(|p| {
                    std::fs::remove_file(&p).ok();
                });
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(())
    }

    /// Loads persisted state from disk, returning the token map
    ///
    /// If `links/` dir exists, reads each `.json` file
    pub(crate) fn load_state(
        dir: &std::path::Path,
    ) -> Result<HashMap<String, DownloadItem>, Box<dyn std::error::Error + Send + Sync>> {
        let links_dir = dir.join(LINKS_DIR);
        // --------------------------------------------------
        // make the directory if it doesn't exist
        // --------------------------------------------------
        match (links_dir.is_dir(), links_dir.exists()) {
            (false, false) => {
                std::fs::create_dir(&links_dir).ok();
            }
            (false, true) => {
                tracing::error!("State directory {links_dir:?} exists but is not a directory");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotADirectory,
                    "links/ directory exists but is not a directory",
                )
                .into());
            }
            (true, _) => {}
        }
        // --------------------------------------------------
        // per-link files, to read and store
        // --------------------------------------------------
        let mut links = HashMap::new();
        for entry in std::fs::read_dir(&links_dir)?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            match std::fs::read_to_string(&path) {
                Ok(json) => match serde_json::from_str::<PersistedDownloadItem>(&json) {
                    Ok(persisted) => {
                        tracing::debug!("Loaded link file {path:?}");
                        links.insert(stem, persisted.into());
                    }
                    Err(e) => tracing::warn!(
                        "Skipping malformed json file (expecting link) {path:?}: {e}"
                    ),
                },
                Err(e) => tracing::warn!("Failed to read link file {path:?}: {e}"),
            }
        }
        // --------------------------------------------------
        // if no valid links found, return an error
        // --------------------------------------------------
        match links.is_empty() {
            false => Ok(links),
            true => Err("No valid link files found".into()),
        }
    }
}
