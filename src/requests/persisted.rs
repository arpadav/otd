//! Persisted (disk-serialized) representation of a download token
//!
//! This type is used only on the server side for saving and loading state
//! between restarts. Each active token is written as a JSON file in the
//! `links/` subdirectory of the data directory
//!
//! The [`From`] conversion from [`PersistedDownloadItem`] into
//! [`DownloadItem`][crate::state::DownloadItem] lives in `state.rs`
//! alongside `DownloadItem`
//!
//! Author: aav

use {crate::core::archive::CompressionType, crate::state::DownloadItem, std::path::PathBuf};

/// Current persistence format version
pub(crate) const STATE_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
/// Serializable representation of a single download token for disk persistence
///
/// Converted from a live [`DownloadItem`][crate::state::DownloadItem] via
/// [`PersistedDownloadItem::from_download_item`], and back via the
/// [`From`] impl in `state.rs`
pub(crate) struct PersistedDownloadItem {
    /// Format version for forward compatibility
    pub(crate) version: u32,
    /// Unix timestamp when this item was saved
    pub(crate) saved_at: u64,
    /// Paths to the source files for this download
    pub(crate) paths: Vec<PathBuf>,
    /// Whether this download represents multiple files (vs a single file)
    pub(crate) is_multi_file: bool,
    /// Human-readable name for this download
    pub(crate) name: String,
    /// The maximum number of times this download can be downloaded
    pub(crate) max_downloads: u32,
    /// The number of times this download has been downloaded
    pub(crate) download_count: u32,
    /// The number of seconds until this download expires, if any
    pub(crate) expires_in_seconds: Option<u64>,
    /// The number of seconds since this download was created
    pub(crate) created_ago_seconds: u64,
    /// The compression type used for this download
    pub(crate) compression: CompressionType,
}

/// [`PersistedDownloadItem`] implementation
impl PersistedDownloadItem {
    /// Converts a live [`DownloadItem`] into its serializable form
    ///
    /// `now` is passed in from the caller so multiple items can be snapshotted
    /// at a consistent point in time
    pub(crate) fn from_download_item(item: &DownloadItem, now: std::time::Instant) -> Self {
        let count = item
            .download_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let expires_in_seconds = item
            .expires_at
            .filter(|&e| now < e)
            .map(|e| e.duration_since(now).as_secs());
        let created_ago_seconds = now.duration_since(item.created_at).as_secs();
        let saved_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
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

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn persisted_item_serde_roundtrip() {
        let item = PersistedDownloadItem {
            version: STATE_VERSION,
            saved_at: 1_700_000_000,
            paths: vec![PathBuf::from("/data/file.txt")],
            is_multi_file: false,
            name: "file.txt".into(),
            max_downloads: 1,
            download_count: 0,
            expires_in_seconds: Some(3600),
            created_ago_seconds: 10,
            compression: CompressionType::Zip,
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: PersistedDownloadItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.version, STATE_VERSION);
        assert_eq!(decoded.name, "file.txt");
        assert_eq!(decoded.expires_in_seconds, Some(3600));
        assert_eq!(decoded.compression, CompressionType::Zip);
    }

    #[test]
    fn persisted_item_no_expiry_roundtrip() {
        let item = PersistedDownloadItem {
            version: STATE_VERSION,
            saved_at: 1_700_000_000,
            paths: vec![PathBuf::from("/data/archive.zip")],
            is_multi_file: false,
            name: "archive.zip".into(),
            max_downloads: 5,
            download_count: 2,
            expires_in_seconds: None,
            created_ago_seconds: 60,
            compression: CompressionType::Zip,
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: PersistedDownloadItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.expires_in_seconds, None);
        assert_eq!(decoded.download_count, 2);
    }
}
