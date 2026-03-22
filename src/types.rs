//! Core data types and structures for the OTD server.
//!
//! This module defines the main data structures used throughout the application,
//! including download items, request/response types, and shared application state.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smol::lock::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicU32};

/// Per-link storage subdirectory within the data directory.
const LINKS_DIR: &str = "links";
/// Current persistence format version.
const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone)]
/// Tracks the lifecycle of an archive for a download item.
pub(crate) enum ArchiveState {
    /// Single-file download - no archive needed
    NotNeeded,
    /// Archive is being created in the background
    Preparing,
    /// Archive is ready and cached at the given path
    Ready(PathBuf),
    /// Archive creation failed
    Failed(String),
}

#[derive(Debug)]
/// Represents a downloadable item with one or more files/folders.
///
/// Each download item is associated with a unique token and can contain
/// multiple paths that will be served as a single download (zip for multiple items).
///
pub(crate) struct DownloadItem {
    /// List of file/folder paths included in this download
    pub(crate) paths: Vec<PathBuf>,
    /// Whether this download contains multiple files/folders (true if paths.len() > 1)
    pub(crate) is_multi_file: bool,
    /// Display name for the download (e.g., "my-files.zip" or "document.pdf")
    pub(crate) name: String,
    /// Maximum allowed downloads before the link becomes invalid
    pub(crate) max_downloads: u32,
    /// Current download count for this item
    pub(crate) download_count: AtomicU32,
    /// Optional expiration time for the download link (None if it does not expire)
    pub(crate) expires_at: Option<std::time::Instant>,
    /// When this download item was created
    pub(crate) created_at: std::time::Instant,
    /// Compression format for archive downloads.
    pub(crate) compression: crate::handlers::download::CompressionType,
    /// Archive preparation state (interior-mutable; never held across .await)
    pub(crate) archive_state: smol::lock::RwLock<ArchiveState>,
    /// Number of downloads currently being served (file read in progress).
    /// Cache files must not be deleted while this is > 0.
    pub(crate) active_serving: AtomicU32,
}
/// [`DownloadItem`] implementation
impl DownloadItem {
    #[inline(always)]
    /// Returns the archive cache path if the archive state is [`ArchiveState::Ready`].
    ///
    /// Uses a non-blocking `try_read` so it never stalls callers.
    pub(crate) fn cache_path(&self) -> Option<PathBuf> {
        self.archive_state.try_read().and_then(|s| match &*s {
            ArchiveState::Ready(p) => Some(p.clone()),
            _ => None,
        })
    }

    #[inline(always)]
    /// Whether it is safe to remove this item's cache file right now.
    ///
    /// Returns `false` if any download is actively being served (file read
    /// in progress), since deleting the cache mid-read would cause the
    /// download to fail.
    pub(crate) fn can_remove_cache(&self) -> bool {
        self.active_serving
            .load(std::sync::atomic::Ordering::Acquire)
            == 0
    }

    /// Removes the archive cache file from disk if no download is in progress.
    ///
    /// # Returns
    ///
    /// * `true` if the file was removed or didn't exist.
    /// * `false` if skipped because a download is in progress.
    pub(crate) fn remove_cache_file(&self) -> bool {
        if !self.can_remove_cache() {
            tracing::debug!("Skipping cache removal: download in progress");
            return false;
        }
        self.cache_path()
            .as_ref()
            .map(crate::handlers::remove_cache_file);
        true
    }
}
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
            RwLock::new(ArchiveState::NotNeeded)
        } else {
            // --------------------------------------------------
            // active multi-file - will be updated after checking
            // cache or re-creating
            // --------------------------------------------------
            RwLock::new(ArchiveState::Preparing)
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

#[derive(Debug, Deserialize)]
/// Query parameters for download requests.
///
/// Used to parse the `?k=<token>` parameter from download URLs.
///
/// # Examples
///
/// ```rust
/// use otd::types::DownloadQuery;
///
/// let query = DownloadQuery {
///     k: "550e8400-e29b-41d4-a716-446655440000".to_string(),
/// };
/// ```
pub struct DownloadQuery {
    /// The unique token identifying the download
    pub k: String,
}

#[derive(Debug, Deserialize)]
/// Request payload for generating new download links.
///
/// Contains the list of file paths to include and an optional custom name.
///
/// # Examples
///
/// ```rust
/// use otd::types::GenerateRequest;
///
/// let request = GenerateRequest {
///     paths: vec!["folder1".to_string(), "file.txt".to_string()],
///     name: Some("my-download.zip".to_string()),
///     max_downloads: Some(5),
///     expires_in_seconds: Some(3600),
///     format: Default::default(),
/// };
///
/// assert_eq!(request.paths.len(), 2);
/// ```
pub struct GenerateRequest {
    /// List of file paths to include in the download.
    pub paths: Vec<String>,
    /// Optional custom name for the download archive.
    pub name: Option<String>,
    /// Optional maximum number of downloads allowed.
    pub max_downloads: Option<u32>,
    /// Optional number of seconds until the download expires.
    pub expires_in_seconds: Option<u64>,

    #[serde(default)]
    /// Archive format (defaults to zip when absent).
    pub format: crate::handlers::download::CompressionType,
}

#[derive(Debug, Serialize)]
/// Represents a file or folder in the file browser.
///
/// Used in API responses to display directory contents in the web interface.
///
/// # Examples
///
/// ```rust
/// use otd::types::FileItem;
///
/// let file = FileItem {
///     name: "document.pdf".to_string(),
///     path: "documents/document.pdf".to_string(),
///     is_dir: false,
///     size: Some(1024),
/// };
/// ```
pub struct FileItem {
    /// Display name of the file/folder
    pub name: String,
    /// Relative path from the base directory
    pub path: String,
    /// Whether this item is a directory
    pub is_dir: bool,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
/// Response payload when a download link is successfully generated.
///
/// Contains the unique token and the full download URL.
///
/// # Examples
///
/// ```rust
/// use otd::types::GenerateResponse;
///
/// let response = GenerateResponse {
///     token: "550e8400-e29b-41d4-a716-446655440000".to_string(),
///     download_url: "http://localhost:15205/my-file.txt?k=550e8400-e29b-41d4-a716-446655440000".to_string(),
/// };
/// ```
pub struct GenerateResponse {
    /// Unique identifier for this download
    pub token: String,
    /// Complete URL for downloading the file(s)
    pub download_url: String,
}

#[derive(Debug, Serialize)]
/// Represents a staged file in the web interface.
///
/// Used to track files that have been selected but not yet turned into a download link.
///
/// # Examples
///
/// ```rust
/// use otd::types::StagedFile;
///
/// let staged = StagedFile {
///     path: "documents/report.pdf".to_string(),
///     name: "report.pdf".to_string(),
///     is_dir: false,
///     size: Some(2048),
/// };
/// ```
pub struct StagedFile {
    /// Relative path from the base directory
    pub path: String,
    /// Display name of the file/folder
    pub name: String,
    /// Whether this item is a directory
    pub is_dir: bool,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
/// Dashboard statistics response payload.
///
/// Returned by the `/api/stats` endpoint with aggregate token and download metrics.
pub(crate) struct StatsResponse {
    /// Number of links that are still valid and have remaining downloads
    pub(crate) active_links: u32,
    /// Number of links that have reached their download limit
    pub(crate) used_links: u32,
    /// Number of links that have passed their expiration time
    pub(crate) expired_links: u32,
    /// Total downloads across all links
    pub(crate) total_downloads: u64,
    /// Server uptime in seconds
    pub(crate) uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
/// Represents a single token in the `/api/links` listing.
pub(crate) struct TokenListItem {
    /// Unique token identifier
    pub(crate) token: String,
    /// Display name for the download
    pub(crate) name: String,
    /// Whether this download contains multiple files/folders
    pub(crate) is_multi_file: bool,
    /// Current download count
    pub(crate) download_count: u32,
    /// Maximum allowed downloads
    pub(crate) max_downloads: u32,
    /// Remaining downloads before the link becomes invalid
    pub(crate) remaining_downloads: u32,
    /// Whether the token has expired
    pub(crate) expired: bool,
    /// Seconds until expiration (None if already expired or no expiry set)
    pub(crate) expires_in_seconds: Option<u64>,
    /// Full download URL
    pub(crate) download_url: String,
    /// List of file/folder paths included in this download
    pub(crate) paths: Vec<String>,
    /// Current archive preparation status
    pub(crate) archive_status: String,
    /// Whether all source files still exist on disk
    pub(crate) source_exists: bool,
}

#[derive(Debug, Serialize)]
/// Response payload for bulk-delete and single-delete operations.
pub(crate) struct BulkDeleteResponse {
    /// Number of links removed
    pub(crate) removed: usize,
}

#[derive(Serialize, Deserialize)]
/// Serializable representation of a single download token.
pub(crate) struct PersistedDownloadItem {
    /// Format version for forward compatibility.
    pub(crate) version: u32,
    /// Unix timestamp when this item was saved.
    pub(crate) saved_at: u64,
    /// Paths to the source files for this download.
    pub(crate) paths: Vec<PathBuf>,
    /// Whether this download represents multiple files (vs a single archive).
    pub(crate) is_multi_file: bool,
    /// Human-readable name for this download.
    pub(crate) name: String,
    /// The maximum number of times this download can be downloaded.
    pub(crate) max_downloads: u32,
    /// The number of times this download has been downloaded.
    pub(crate) download_count: u32,
    /// The number of seconds until this download expires, if any.
    pub(crate) expires_in_seconds: Option<u64>,
    /// The number of seconds since this download was created.
    pub(crate) created_ago_seconds: u64,
    /// The compression type used for this download.
    pub(crate) compression: crate::handlers::download::CompressionType,
}
/// [`PersistedDownloadItem`] implementation
impl PersistedDownloadItem {
    /// Converts a live [`DownloadItem`] into its serializable form
    ///
    /// Need this, so that `now` can be passed in from the caller.
    pub(crate) fn from_download_item(item: &DownloadItem, now: std::time::Instant) -> Self {
        // --------------------------------------------------
        // get the download count, from atomic
        // --------------------------------------------------
        let count = item
            .download_count
            .load(std::sync::atomic::Ordering::Relaxed);
        // --------------------------------------------------
        // calculate the expired, created, and saved at times
        // --------------------------------------------------
        let expires_in_seconds = item
            .expires_at
            .filter(|&e| now < e)
            .map(|e| e.duration_since(now).as_secs());
        let created_ago_seconds = now.duration_since(item.created_at).as_secs();
        let saved_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Self {
            version: STATE_VERSION,
            saved_at,
            paths: item.paths.clone(),
            is_multi_file: item.is_multi_file,
            name: item.name.clone(),
            max_downloads: item.max_downloads,
            download_count: count,
            expires_in_seconds,
            created_ago_seconds,
            compression: item.compression,
        }
    }
}

/// Shared application state containing configuration and active downloads.
///
/// This structure is shared between all request handlers and contains the
/// core application data including active download links and configuration.
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
    /// Active login sessions: token → creation time
    pub(crate) sessions: RwLock<HashMap<String, std::time::Instant>>,
    /// Server start time for uptime tracking
    pub(crate) started_at: std::time::Instant,
    /// Set when links change; cleared after state is persisted.
    pub(crate) dirty: std::sync::atomic::AtomicBool,
}
/// [`AppState`] implementation of [`Default`]
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
/// [`AppState`] implementation
impl AppState {
    /// Creates a new application state.
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
            sessions: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Creates an `AppState` pre-loaded with persisted links.
    pub(crate) fn with_links(links: HashMap<String, DownloadItem>) -> Self {
        Self {
            links: RwLock::new(links),
            sessions: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Marks the state as dirty so it will be persisted on the next save cycle.
    pub(crate) fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::Release);
    }

    /// Saves the current token state to disk as individual per-link JSON files.
    ///
    /// Each token is written as `links/<token>.json` via atomic `.tmp` rename.
    /// After writing, orphan files (links no longer in memory) are removed.
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

    /// Loads persisted state from disk, returning the token map.
    ///
    /// If `links/` dir exists, reads each `.json` file.
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

    /// Clears all links from memory and deletes the `links/` directory.
    pub(crate) async fn clear_links(&self, dir: &std::path::Path) -> usize {
        let mut links = self.links.write().await;
        let count = links.len();
        // --------------------------------------------------
        // remove archive cache files for all links
        // --------------------------------------------------
        cfg_if::cfg_if! {
            if #[cfg(feature = "rayon")] {
                links
                    .par_iter()
                    .map(|(_, v)| v)
                    .for_each(|item| { item.remove_cache_file(); });
            } else {
                links
                    .values()
                    .for_each(|item| { item.remove_cache_file(); });
            }
        }
        links.clear();
        drop(links);
        let links_dir = dir.join(LINKS_DIR);
        let _ = std::fs::remove_dir_all(&links_dir);
        let _ = std::fs::create_dir_all(&links_dir);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_app_state_creation() {
        let _state = AppState::new();
    }

    #[test]
    fn test_download_item() {
        let item = DownloadItem {
            paths: vec![PathBuf::from("test.txt")],
            is_multi_file: false,
            name: "test.txt".to_string(),
            max_downloads: 3,
            download_count: AtomicU32::new(0),
            expires_at: None,
            created_at: std::time::Instant::now(),
            compression: crate::handlers::download::CompressionType::Zip,
            archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
            active_serving: AtomicU32::new(0),
        };

        assert_eq!(item.paths.len(), 1);
        assert!(!item.is_multi_file);
        assert_eq!(item.download_count.load(Ordering::Relaxed), 0);
        assert_eq!(item.name, "test.txt");
        assert_eq!(item.max_downloads, 3);
    }

    #[test]
    fn test_persisted_roundtrip() {
        let item = DownloadItem {
            paths: vec![PathBuf::from("/tmp/file.txt")],
            is_multi_file: false,
            name: "file.txt".to_string(),
            max_downloads: 5,
            download_count: AtomicU32::new(2),
            expires_at: Some(std::time::Instant::now() + std::time::Duration::from_secs(600)),
            created_at: std::time::Instant::now() - std::time::Duration::from_secs(30),
            compression: crate::handlers::download::CompressionType::TarGz,
            archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
            active_serving: AtomicU32::new(0),
        };

        let now = std::time::Instant::now();
        let persisted = PersistedDownloadItem::from_download_item(&item, now);
        assert_eq!(persisted.download_count, 2);
        assert_eq!(persisted.max_downloads, 5);
        assert!(persisted.expires_in_seconds.is_some());
        assert!(persisted.created_ago_seconds >= 29);

        let restored: DownloadItem = persisted.into();
        assert_eq!(restored.name, "file.txt");
        assert_eq!(restored.max_downloads, 5);
        assert_eq!(restored.download_count.load(Ordering::Relaxed), 2);
        assert!(restored.expires_at.is_some());
    }

    #[test]
    fn test_save_load_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();

        // Insert a token
        smol::block_on(async {
            let mut links = state.links.write().await;
            links.insert(
                "test-token".to_string(),
                DownloadItem {
                    paths: vec![PathBuf::from("/tmp/a.txt")],
                    is_multi_file: false,
                    name: "a.txt".to_string(),
                    max_downloads: 3,
                    download_count: AtomicU32::new(1),
                    expires_at: Some(
                        std::time::Instant::now() + std::time::Duration::from_secs(300),
                    ),
                    created_at: std::time::Instant::now(),
                    compression: crate::handlers::download::CompressionType::Zip,
                    archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
                    active_serving: AtomicU32::new(0),
                },
            );
        });
        smol::block_on(state.save_state(dir.path())).unwrap();
        assert!(dir.path().join("links").is_dir());
        assert!(dir.path().join("links/test-token.json").is_file());
        let loaded = AppState::load_state(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        let item = loaded.get("test-token").unwrap();
        assert_eq!(item.name, "a.txt");
        assert_eq!(item.download_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_load_state_includes_all_links() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();

        smol::block_on(async {
            let mut links = state.links.write().await;
            // Fully used token
            links.insert(
                "used-token".to_string(),
                DownloadItem {
                    paths: vec![PathBuf::from("/tmp/b.txt")],
                    is_multi_file: false,
                    name: "b.txt".to_string(),
                    max_downloads: 1,
                    download_count: AtomicU32::new(1),
                    expires_at: None,
                    created_at: std::time::Instant::now(),
                    compression: crate::handlers::download::CompressionType::Zip,
                    archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
                    active_serving: AtomicU32::new(0),
                },
            );
            // Active token
            links.insert(
                "active-token".to_string(),
                DownloadItem {
                    paths: vec![PathBuf::from("/tmp/c.txt")],
                    is_multi_file: false,
                    name: "c.txt".to_string(),
                    max_downloads: 5,
                    download_count: AtomicU32::new(0),
                    expires_at: None,
                    created_at: std::time::Instant::now(),
                    compression: crate::handlers::download::CompressionType::Zip,
                    archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
                    active_serving: AtomicU32::new(0),
                },
            );
        });

        smol::block_on(state.save_state(dir.path())).unwrap();
        let loaded = AppState::load_state(dir.path()).unwrap();
        // Both links are loaded (including used ones) for display
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains_key("active-token"));
        assert!(loaded.contains_key("used-token"));
    }

    #[test]
    fn test_clear_links() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();

        smol::block_on(async {
            let mut links = state.links.write().await;
            links.insert(
                "tok1".to_string(),
                DownloadItem {
                    paths: vec![PathBuf::from("/tmp/x.txt")],
                    is_multi_file: false,
                    name: "x.txt".to_string(),
                    max_downloads: 1,
                    download_count: AtomicU32::new(0),
                    expires_at: None,
                    created_at: std::time::Instant::now(),
                    compression: crate::handlers::download::CompressionType::Zip,
                    archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
                    active_serving: AtomicU32::new(0),
                },
            );
        });
        smol::block_on(state.save_state(dir.path())).unwrap();
        assert!(dir.path().join("links/tok1.json").is_file());
        let removed = smol::block_on(state.clear_links(dir.path()));
        assert_eq!(removed, 1);
        assert_eq!(smol::block_on(state.links.read()).len(), 0);
        assert!(dir.path().join("links").is_dir());
        assert_eq!(
            std::fs::read_dir(dir.path().join("links")).unwrap().count(),
            0
        );
    }

    #[test]
    fn test_save_removes_orphan_files() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();

        smol::block_on(async {
            let mut links = state.links.write().await;
            links.insert(
                "keep-me".to_string(),
                DownloadItem {
                    paths: vec![PathBuf::from("/tmp/k.txt")],
                    is_multi_file: false,
                    name: "k.txt".to_string(),
                    max_downloads: 1,
                    download_count: AtomicU32::new(0),
                    expires_at: None,
                    created_at: std::time::Instant::now(),
                    compression: crate::handlers::download::CompressionType::Zip,
                    archive_state: smol::lock::RwLock::new(ArchiveState::NotNeeded),
                    active_serving: AtomicU32::new(0),
                },
            );
        });
        smol::block_on(state.save_state(dir.path())).unwrap();
        std::fs::write(dir.path().join("links/orphan.json"), "{}").unwrap();
        assert!(dir.path().join("links/orphan.json").exists());
        smol::block_on(state.save_state(dir.path())).unwrap();
        assert!(!dir.path().join("links/orphan.json").exists());
        assert!(dir.path().join("links/keep-me.json").exists());
    }

    #[test]
    fn test_dirty_flag() {
        let state = AppState::new();
        assert!(!state.dirty.load(Ordering::Relaxed));
        state.mark_dirty();
        assert!(state.dirty.load(Ordering::Relaxed));
    }

    #[test]
    fn test_file_item_serialization() {
        let file = FileItem {
            name: "test.txt".to_string(),
            path: "folder/test.txt".to_string(),
            is_dir: false,
            size: Some(1024),
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("1024"));
        assert!(json.contains("false"));
    }
}
