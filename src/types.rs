//! Core data types and structures for the OTD server.
//!
//! This module defines the main data structures used throughout the application,
//! including download items, request/response types, and shared application state.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use serde::{Deserialize, Serialize};
use smol::lock::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicU32};

/// State file name within the data directory.
const STATE_FILE: &str = "state.json";
/// Current persistence format version.
const STATE_VERSION: u32 = 1;

#[derive(Debug)]
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
    pub paths: Vec<String>,
    pub name: Option<String>,
    pub max_downloads: Option<u32>,
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
    /// Number of tokens that are still valid and have remaining downloads
    pub(crate) active_tokens: u32,
    /// Number of tokens that have reached their download limit
    pub(crate) used_tokens: u32,
    /// Number of tokens that have passed their expiration time
    pub(crate) expired_tokens: u32,
    /// Total downloads across all tokens
    pub(crate) total_downloads: u64,
    /// Server uptime in seconds
    pub(crate) uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
/// Represents a single token in the `/api/tokens` listing.
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
}

#[derive(Debug, Serialize)]
/// Response payload for bulk-delete and single-delete operations.
pub(crate) struct BulkDeleteResponse {
    /// Number of tokens removed
    pub(crate) removed: usize,
}

/// Serializable snapshot of all tokens for persistence across restarts.
#[derive(Serialize, Deserialize)]
pub(crate) struct PersistedState {
    /// Format version for forward compatibility.
    pub(crate) version: u32,
    /// Unix timestamp when the state was saved.
    pub(crate) saved_at: u64,
    /// All active download tokens.
    pub(crate) tokens: HashMap<String, PersistedDownloadItem>,
}

/// Serializable representation of a single download token.
#[derive(Serialize, Deserialize)]
pub(crate) struct PersistedDownloadItem {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) is_multi_file: bool,
    pub(crate) name: String,
    pub(crate) max_downloads: u32,
    pub(crate) download_count: u32,
    pub(crate) expires_in_seconds: Option<u64>,
    pub(crate) created_ago_seconds: u64,
    pub(crate) compression: crate::handlers::download::CompressionType,
}

impl PersistedDownloadItem {
    /// Converts a live [`DownloadItem`] into its serializable form.
    pub(crate) fn from_download_item(item: &DownloadItem, now: std::time::Instant) -> Self {
        let count = item
            .download_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let expires_in_seconds = item.expires_at.and_then(|e| {
            if now < e {
                Some(e.duration_since(now).as_secs())
            } else {
                None
            }
        });
        let created_ago_seconds = now.duration_since(item.created_at).as_secs();
        Self {
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

    /// Converts back into a live [`DownloadItem`].
    pub(crate) fn into_download_item(self) -> DownloadItem {
        let now = std::time::Instant::now();
        let expires_at = self
            .expires_in_seconds
            .map(|secs| now + std::time::Duration::from_secs(secs));
        let created_at = now - std::time::Duration::from_secs(self.created_ago_seconds);
        let archive_state = if self.is_multi_file {
            // Will be updated after checking cache or re-creating
            RwLock::new(ArchiveState::Preparing)
        } else {
            RwLock::new(ArchiveState::NotNeeded)
        };
        DownloadItem {
            paths: self.paths,
            is_multi_file: self.is_multi_file,
            name: self.name,
            max_downloads: self.max_downloads,
            download_count: AtomicU32::new(self.download_count),
            expires_at,
            created_at,
            compression: self.compression,
            archive_state,
        }
    }
}

/// Shared application state containing configuration and active downloads.
///
/// This structure is shared between all request handlers and contains the
/// core application data including active download tokens and configuration.
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
    pub(crate) tokens: RwLock<HashMap<String, DownloadItem>>,
    /// Active login sessions: token → creation time
    pub(crate) sessions: RwLock<HashMap<String, std::time::Instant>>,
    /// Server start time for uptime tracking
    pub(crate) started_at: std::time::Instant,
    /// Set when tokens change; cleared after state is persisted.
    pub(crate) dirty: std::sync::atomic::AtomicBool,
}
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
            tokens: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Creates an `AppState` pre-loaded with persisted tokens.
    pub(crate) fn with_tokens(tokens: HashMap<String, DownloadItem>) -> Self {
        Self {
            tokens: RwLock::new(tokens),
            sessions: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
            dirty: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Marks the state as dirty so it will be persisted on the next save cycle.
    pub(crate) fn mark_dirty(&self) {
        self.dirty.store(true, std::sync::atomic::Ordering::Release);
    }

    /// Saves the current token state to disk (atomic write).
    pub(crate) async fn save_state(
        &self,
        dir: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = std::time::Instant::now();
        let tokens = self.tokens.read().await;
        let persisted_tokens: HashMap<String, PersistedDownloadItem> = tokens
            .iter()
            .map(|(k, v)| (k.clone(), PersistedDownloadItem::from_download_item(v, now)))
            .collect();
        drop(tokens);

        let state = PersistedState {
            version: STATE_VERSION,
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            tokens: persisted_tokens,
        };

        let json = serde_json::to_string_pretty(&state)?;
        let path = dir.join(STATE_FILE);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Loads persisted state from disk, returning the token map.
    ///
    /// Skips tokens that have already expired.
    pub(crate) fn load_state(
        dir: &std::path::Path,
    ) -> Result<HashMap<String, DownloadItem>, Box<dyn std::error::Error + Send + Sync>> {
        let path = dir.join(STATE_FILE);
        let json = std::fs::read_to_string(&path)?;
        let state: PersistedState = serde_json::from_str(&json)?;
        let mut tokens = HashMap::new();
        for (key, persisted) in state.tokens {
            // Skip already-expired tokens
            if persisted.expires_in_seconds == Some(0) {
                continue;
            }
            // Skip fully-used tokens
            if persisted.download_count >= persisted.max_downloads {
                continue;
            }
            tokens.insert(key, persisted.into_download_item());
        }
        Ok(tokens)
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
        };

        let now = std::time::Instant::now();
        let persisted = PersistedDownloadItem::from_download_item(&item, now);
        assert_eq!(persisted.download_count, 2);
        assert_eq!(persisted.max_downloads, 5);
        assert!(persisted.expires_in_seconds.is_some());
        assert!(persisted.created_ago_seconds >= 29);

        let restored = persisted.into_download_item();
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
            let mut tokens = state.tokens.write().await;
            tokens.insert(
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
                },
            );
        });

        // Save
        smol::block_on(state.save_state(dir.path())).unwrap();

        // Load
        let loaded = AppState::load_state(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        let item = loaded.get("test-token").unwrap();
        assert_eq!(item.name, "a.txt");
        assert_eq!(item.download_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_load_state_skips_used_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();

        smol::block_on(async {
            let mut tokens = state.tokens.write().await;
            // Fully used token
            tokens.insert(
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
                },
            );
            // Active token
            tokens.insert(
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
                },
            );
        });

        smol::block_on(state.save_state(dir.path())).unwrap();
        let loaded = AppState::load_state(dir.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("active-token"));
        assert!(!loaded.contains_key("used-token"));
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
