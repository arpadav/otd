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
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU32},
};

/// Tracks the lifecycle of a zip archive for a download item.
#[derive(Debug)]
pub enum ZipState {
    /// Single-file download - no zip needed
    NotNeeded,
    /// Zip is being created in the background
    Preparing,
    /// Zip is ready and cached at the given path
    Ready(PathBuf),
    /// Zip creation failed
    Failed(String),
}

#[derive(Debug)]
/// Represents a downloadable item with one or more files/folders.
///
/// Each download item is associated with a unique token and can contain
/// multiple paths that will be served as a single download (zip for multiple items).
///
pub struct DownloadItem {
    /// List of file/folder paths included in this download
    pub paths: Vec<PathBuf>,
    /// Whether this download contains multiple files/folders (true if paths.len() > 1)
    pub is_multi_file: bool,
    /// Display name for the download (e.g., "my-files.zip" or "document.pdf")
    pub name: String,
    /// Maximum allowed downloads before the link becomes invalid
    pub max_downloads: u32,
    /// Current download count for this item
    pub download_count: AtomicU32,
    /// Optional expiration time for the download link (None if it does not expire)
    pub expires_at: Option<std::time::Instant>,
    /// When this download item was created
    pub created_at: std::time::Instant,
    /// Zip preparation state (interior-mutable; never held across .await)
    pub zip_state: std::sync::RwLock<ZipState>,
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
/// };
///
/// assert_eq!(request.paths.len(), 2);
/// ```
pub struct GenerateRequest {
    pub paths: Vec<String>,
    pub name: Option<String>,
    pub max_downloads: Option<u32>,
    pub expires_in_seconds: Option<u64>,
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

/// Shared application state containing configuration and active downloads.
///
/// This structure is shared between all request handlers and contains the
/// core application data including active download tokens and configuration.
///
/// # Examples
///
/// ```rust
/// use otd::types::AppState;
/// use std::path::PathBuf;
///
/// let state = AppState::new(PathBuf::from("/home/user/files"));
/// ```
pub struct AppState {
    /// Map of active download tokens to their corresponding items
    pub tokens: RwLock<HashMap<String, DownloadItem>>,
    /// Whether one-time download enforcement is enabled
    pub one_time_enabled: AtomicBool,
    /// Base directory path for file serving
    pub base_path: PathBuf,
    /// Active login sessions: token → creation time
    pub sessions: RwLock<HashMap<String, std::time::Instant>>,
    /// Server start time for uptime tracking
    pub started_at: std::time::Instant,
}

impl AppState {
    /// Creates a new application state with the specified base path.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for file serving and browsing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::types::AppState;
    /// use std::path::PathBuf;
    ///
    /// let state = AppState::new(PathBuf::from("/var/files"));
    /// assert_eq!(state.base_path, PathBuf::from("/var/files"));
    /// ```
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
            one_time_enabled: AtomicBool::new(true),
            base_path,
            sessions: RwLock::new(HashMap::new()),
            started_at: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_app_state_creation() {
        let base_path = PathBuf::from("/test/path");
        let state = AppState::new(base_path.clone());

        assert_eq!(state.base_path, base_path);
        assert!(state.one_time_enabled.load(Ordering::Relaxed));
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
            zip_state: std::sync::RwLock::new(ZipState::NotNeeded),
        };

        assert_eq!(item.paths.len(), 1);
        assert!(!item.is_multi_file);
        assert_eq!(item.download_count.load(Ordering::Relaxed), 0);
        assert_eq!(item.name, "test.txt");
        assert_eq!(item.max_downloads, 3);
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
