//! File browsing endpoint and types
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::api::error::ApiError;
use crate::config;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};

// --------------------------------------------------
// types
// --------------------------------------------------
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
/// Represents a file or folder entry in the file browser
///
/// # Example
///
/// ```rust,ignore
/// use otd::api::routes::browse::FileItem;
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

#[derive(Deserialize)]
/// Query parameters for the browse endpoint
pub(crate) struct BrowseQuery {
    #[serde(default)]
    /// Relative path to browse (empty string = serve root)
    path: String,
}

/// Handles `GET /api/browse`
///
/// Returns a JSON array of [`FileItem`] entries in the requested directory,
/// sorted with directories first (alphabetically), then files (alphabetically)
/// If `path` is empty the configured base directory is used as the root
/// Inserts a synthetic `..` entry when the requested path is not the root,
/// allowing clients to navigate upward. Requests that would escape the base
/// directory via `../` traversal or symlink are rejected with 403 Forbidden
/// Returns 404 if the resolved directory cannot be read
///
/// # Arguments
///
/// * `params` - Query string parsed into [`BrowseQuery`]; the `path` field is
///   relative to the server's configured base directory
pub async fn browse(Query(params): Query<BrowseQuery>) -> Result<Json<Vec<FileItem>>, ApiError> {
    // --------------------------------------------------
    // acquire config and extract base path and request path
    // --------------------------------------------------
    let cfg = config::config().read().await;
    let base_path = cfg.canonical_base_path.clone();
    let path = params.path;
    // --------------------------------------------------
    // resolve the full filesystem path for the request:
    // use base_path for empty path, otherwise use safe_join
    // which canonicalizes and verifies containment, blocking
    // both `../` traversal and symlink escapes
    // --------------------------------------------------
    let full_path = if path.is_empty() {
        base_path.clone()
    } else {
        match cfg.safe_join(&path) {
            Some(p) => p,
            None => {
                tracing::warn!("Browse: path traversal/symlink escape blocked for '{path}'");
                return Err(ApiError::Forbidden("Forbidden".into()));
            }
        }
    };
    drop(cfg);
    // --------------------------------------------------
    // accumulate directory entries
    // --------------------------------------------------
    tracing::debug!("Browse request - path: '{path}', full_path: '{full_path:?}'");
    let mut items = Vec::new();
    // --------------------------------------------------
    // add parent directory if not at root
    // --------------------------------------------------
    if full_path != base_path
        && let Some(parent) = full_path.parent()
        && parent.starts_with(&base_path)
    {
        let relative_parent = parent
            .strip_prefix(&base_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        items.push(FileItem {
            name: "..".to_string(),
            path: relative_parent,
            is_dir: true,
            size: None,
        });
    }
    // --------------------------------------------------
    // get entries of the selected dir (non-blocking using tokio)
    // --------------------------------------------------
    match tokio::fs::read_dir(&full_path).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let metadata = entry.metadata().await.ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = if is_dir {
                    None
                } else {
                    metadata.map(|m| m.len())
                };
                let entry_path = entry.path();
                let relative_path = entry_path
                    .strip_prefix(&base_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());
                items.push(FileItem {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: relative_path,
                    is_dir,
                    size,
                });
            }
        }
        Err(e) => {
            tracing::error!("Failed to read directory {full_path:?}: {e}");
            return Err(ApiError::NotFound("Directory not found".into()));
        }
    }
    // --------------------------------------------------
    // sort: directories first, then files, both alphabetically
    // --------------------------------------------------
    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(Json(items))
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
