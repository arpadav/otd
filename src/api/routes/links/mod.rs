//! Link management endpoint types and re-exports
//!
//! Defines the shared request/response types used across all link-management
//! handlers (`generate`, `delete`, `bulk_delete`, `revive`, `list`, `stats`),
//! and re-exports each handler so callers only need to import from this module
//!
//! Author: aav

// --------------------------------------------------
// modules
// --------------------------------------------------
pub mod bulk_delete;
pub mod delete;
pub mod edit;
pub mod generate;
pub mod list;
pub mod revive;
pub mod stats;

// --------------------------------------------------
// re-exports (handlers)
// --------------------------------------------------
pub use bulk_delete::bulk_delete_links;
pub use delete::delete_link;
pub use edit::edit_link;
pub use generate::generate_link;
pub use list::list_links;
pub use revive::revive_link;
pub use stats::stats;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::archive::CompressionType;

// --------------------------------------------------
// external
// --------------------------------------------------
use serde::{Deserialize, Serialize};

// --------------------------------------------------
// types
// --------------------------------------------------
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
/// Represents a single token entry in the `/api/links` listing
///
/// # Example
///
/// ```
/// use otd::api::routes::links::TokenListItem;
///
/// let json = r#"{
///   "token": "abc-123",
///   "name": "report.zip",
///   "is_multi_file": false,
///   "download_count": 1,
///   "max_downloads": 3,
///   "remaining_downloads": 2,
///   "expired": false,
///   "expires_in_seconds": null,
///   "download_url": "http://localhost/dl/report.zip?k=abc-123",
///   "paths": ["/data/report.pdf"],
///   "archive_status": "not_needed",
///   "source_exists": true
/// }"#;
/// let item: TokenListItem = serde_json::from_str(json).unwrap();
/// assert_eq!(item.token, "abc-123");
/// assert_eq!(item.remaining_downloads, 2);
/// ```
pub struct TokenListItem {
    /// Unique token identifier
    pub token: String,

    /// Display name for the download
    pub name: String,

    /// Whether this download contains multiple files or folders
    pub is_multi_file: bool,

    /// Current download count
    pub download_count: u32,

    /// Maximum allowed downloads
    pub max_downloads: u32,

    /// Remaining downloads before the link becomes invalid
    pub remaining_downloads: u32,

    /// Whether the token has expired
    pub expired: bool,

    /// Seconds until expiration (`None` if already expired or no expiry set)
    pub expires_in_seconds: Option<u64>,

    /// Full download URL
    pub download_url: String,

    /// List of file or folder paths included in this download
    pub paths: Vec<String>,

    #[serde(
        deserialize_with = "link_status_deserializer",
        serialize_with = "link_status_serializer"
    )]
    /// Current link archive preparation status
    pub link_status: crate::shared::LinkStatuses,

    /// Whether all source files still exist on disk
    pub source_exists: bool,
}

#[inline(always)]
/// Serialize a [`LinkStatuses`] to a string
fn link_status_serializer<S>(
    status: &crate::shared::LinkStatuses,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(status.as_str())
}

/// Deserialize a [`LinkStatuses`] from a string
fn link_status_deserializer<'de, D>(
    deserializer: D,
) -> Result<crate::shared::LinkStatuses, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    crate::shared::LinkStatuses::parse(&s)
        .ok_or(serde::de::Error::custom("invalid link status {s}"))
}

#[derive(Debug, Deserialize, Serialize)]
/// Dashboard statistics response payload
///
/// # Example
///
/// ```
/// use otd::api::routes::links::StatsResponse;
///
/// let json = r#"{
///   "active_links": 3,
///   "used_links": 1,
///   "expired_links": 2,
///   "total_downloads": 42,
///   "uptime_seconds": 86400
/// }"#;
/// let stats: StatsResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(stats.active_links, 3);
/// assert_eq!(stats.total_downloads, 42);
/// ```
pub struct StatsResponse {
    /// Number of links that are still valid and have remaining downloads
    pub active_links: u32,
    /// Number of links that have reached their download limit
    pub used_links: u32,
    /// Number of links that have passed their expiration time
    pub expired_links: u32,
    /// Total downloads across all links
    pub total_downloads: u64,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize)]
/// Request payload for generating new download links
///
/// # Example
///
/// ```
/// use otd::api::routes::links::GenerateRequest;
///
/// let json = r#"{
///   "paths": ["/home/user/file.txt"],
///   "name": "my-download",
///   "max_downloads": 3,
///   "expires_in_seconds": 3600
/// }"#;
/// let req: GenerateRequest = serde_json::from_str(json).unwrap();
/// assert_eq!(req.paths, vec!["/home/user/file.txt"]);
/// assert_eq!(req.name.as_deref(), Some("my-download"));
/// ```
pub struct GenerateRequest {
    /// List of file paths to include in the download
    pub paths: Vec<String>,
    /// Optional custom name for the download archive
    pub name: Option<String>,
    /// Optional maximum number of downloads allowed
    pub max_downloads: Option<u32>,
    /// Optional number of seconds until the download expires
    pub expires_in_seconds: Option<u64>,
    #[serde(default)]
    /// Archive format (defaults to [`CompressionType::Zip`] when absent)
    pub format: CompressionType,
}

#[derive(Debug, Deserialize, Serialize)]
/// Response payload when a download link is successfully generated
///
/// # Example
///
/// ```
/// use otd::api::routes::links::GenerateResponse;
///
/// let json = r#"{"token":"abc123","download_url":"https://example.com/dl/file.zip?k=abc123"}"#;
/// let resp: GenerateResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(resp.token, "abc123");
/// ```
pub struct GenerateResponse {
    /// Unique identifier for this download
    pub token: String,
    /// Complete URL for downloading the file(s)
    pub download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
/// Response payload for bulk-delete and single-delete operations
///
/// # Example
///
/// ```
/// use otd::api::routes::links::BulkDeleteResponse;
///
/// let json = r#"{"removed":3}"#;
/// let resp: BulkDeleteResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(resp.removed, 3);
/// ```
pub struct BulkDeleteResponse {
    /// Number of links removed
    pub removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::LinkStatuses;

    #[test]
    fn token_list_item_serde_roundtrip() {
        let item = TokenListItem {
            token: "tok-1".into(),
            name: "output.zip".into(),
            is_multi_file: true,
            download_count: 0,
            max_downloads: 1,
            remaining_downloads: 1,
            expired: false,
            expires_in_seconds: Some(3600),
            download_url: "http://localhost/dl/output.zip?k=tok-1".into(),
            paths: vec!["/data/a.txt".into(), "/data/b.txt".into()],
            link_status: LinkStatuses::Active,
            source_exists: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: TokenListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, item);
    }

    #[test]
    fn token_list_item_optional_expiry_roundtrips_none() {
        let item = TokenListItem {
            token: "tok-2".into(),
            name: "file.txt".into(),
            is_multi_file: false,
            download_count: 1,
            max_downloads: 1,
            remaining_downloads: 0,
            expired: false,
            expires_in_seconds: None,
            download_url: "http://localhost/dl/file.txt?k=tok-2".into(),
            paths: vec!["/data/file.txt".into()],
            link_status: LinkStatuses::Expired,
            source_exists: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"expires_in_seconds\":null"));
        let decoded: TokenListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.expires_in_seconds, None);
    }

    #[test]
    fn stats_response_serde_roundtrip() {
        let stats = StatsResponse {
            active_links: 5,
            used_links: 2,
            expired_links: 1,
            total_downloads: 100,
            uptime_seconds: 3600,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let decoded: StatsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.active_links, stats.active_links);
        assert_eq!(decoded.used_links, stats.used_links);
        assert_eq!(decoded.expired_links, stats.expired_links);
        assert_eq!(decoded.total_downloads, stats.total_downloads);
        assert_eq!(decoded.uptime_seconds, stats.uptime_seconds);
    }

    #[test]
    fn generate_request_serde_roundtrip() {
        let req = GenerateRequest {
            paths: vec!["/tmp/file.txt".into()],
            name: Some("archive".into()),
            max_downloads: Some(5),
            expires_in_seconds: Some(7200),
            format: CompressionType::TarGz,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: GenerateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.paths, req.paths);
        assert_eq!(decoded.name, req.name);
        assert_eq!(decoded.max_downloads, req.max_downloads);
        assert_eq!(decoded.expires_in_seconds, req.expires_in_seconds);
        assert_eq!(decoded.format, CompressionType::TarGz);
    }

    #[test]
    fn generate_request_format_defaults_to_zip() {
        let json = r#"{"paths":["/tmp/a.txt"]}"#;
        let req: GenerateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.format, CompressionType::Zip);
    }

    #[test]
    fn generate_response_serde_roundtrip() {
        let resp = GenerateResponse {
            token: "tok-xyz".into(),
            download_url: "http://localhost/dl/f.zip?k=tok-xyz".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GenerateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.token, resp.token);
        assert_eq!(decoded.download_url, resp.download_url);
    }

    #[test]
    fn bulk_delete_response_serde_roundtrip() {
        let resp = BulkDeleteResponse { removed: 7 };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: BulkDeleteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.removed, resp.removed);
    }

    #[test]
    fn bulk_delete_response_zero_removed() {
        let json = r#"{"removed":0}"#;
        let resp: BulkDeleteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.removed, 0);
    }
}
