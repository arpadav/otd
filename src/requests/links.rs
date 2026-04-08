//! Token list item type for the links listing API
//!
//! Used by the [`list_links`][crate::core::links::list_links] server function
//! and the links page client code
//!
//! Author: aav

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
/// Represents a single token entry in the `/api/links` listing
///
/// # Examples
///
/// ```
/// use otd::requests::TokenListItem;
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
    /// Current archive preparation status
    pub archive_status: String,
    /// Whether all source files still exist on disk
    pub source_exists: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            archive_status: "ready".into(),
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
            archive_status: "not_needed".into(),
            source_exists: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"expires_in_seconds\":null"));
        let decoded: TokenListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.expires_in_seconds, None);
    }
}
