//! File-browser item type for the directory browsing API
//!
//! Used by the [`browse`][crate::core::browse::browse] server function
//! and the browse page client code
//!
//! Author: aav

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
/// Represents a file or folder entry in the file browser
///
/// # Examples
///
/// ```
/// use otd::requests::FileItem;
///
/// let json = r#"{"name":"report.pdf","path":"docs/report.pdf","is_dir":false,"size":204800}"#;
/// let item: FileItem = serde_json::from_str(json).unwrap();
/// assert_eq!(item.name, "report.pdf");
/// assert!(!item.is_dir);
/// assert_eq!(item.size, Some(204800));
/// ```
pub struct FileItem {
    /// Display name of the file or folder
    pub name: String,
    /// Relative path from the configured base directory
    pub path: String,
    /// Whether this entry is a directory
    pub is_dir: bool,
    /// File size in bytes (`None` for directories)
    pub size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_item_serde_roundtrip() {
        let item = FileItem {
            name: "data.csv".into(),
            path: "exports/data.csv".into(),
            is_dir: false,
            size: Some(1024),
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: FileItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, item);
    }

    #[test]
    fn file_item_dir_has_no_size() {
        let json = r#"{"name":"images","path":"images","is_dir":true,"size":null}"#;
        let item: FileItem = serde_json::from_str(json).unwrap();
        assert!(item.is_dir);
        assert_eq!(item.size, None);
    }
}
