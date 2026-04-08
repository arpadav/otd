//! File browsing for the admin interface
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use super::prelude::*;
use crate::requests::FileItem;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

/// Returns a sorted list of files/folders in the specified directory
#[get("/api/browse?path")]
pub async fn browse(path: String) -> Result<Vec<FileItem>> {
    let cfg = CONFIG.read().await;
    let base_path = cfg.canonical_base_path.clone();
    drop(cfg);

    let full_path = if path.is_empty() {
        base_path.clone()
    } else {
        // --------------------------------------------------
        // safe_join canonicalizes and verifies containment, blocking both
        // `../` traversal and symlink escapes
        // --------------------------------------------------
        match super::safe_join(&base_path, &path) {
            Some(p) => p,
            None => {
                tracing::warn!("Browse: path traversal/symlink escape blocked for '{path}'");
                return Err(ServerFnError::new("Forbidden").into());
            }
        }
    };
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
    let base_path_owned = base_path.clone();
    match tokio::fs::read_dir(&full_path).await {
        Ok(mut entries) => {
            // --------------------------------------------------
            // if entries are found, extend the items vector
            // --------------------------------------------------
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
                    .strip_prefix(&base_path_owned)
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
            // --------------------------------------------------
            // otherwise, return a 404 response
            // --------------------------------------------------
            tracing::error!("Failed to read directory {full_path:?}: {e}");
            return Err(ServerFnError::new("Directory not found").into());
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
    // --------------------------------------------------
    // return
    // --------------------------------------------------
    Ok(items)
}
