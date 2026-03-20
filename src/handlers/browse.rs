//! File browsing handler for the admin interface.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{config::CONFIG, http::*, prelude::*, types::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use smol::stream::StreamExt;

// --------------------------------------------------
// constants
// --------------------------------------------------
const NUM_FILES_PRE_ALLOCATION: usize = 32;

/// File browsing methods for [`super::Handler`].
impl super::Handler {
    /// Handles file browsing requests for the admin interface.
    ///
    /// Returns a JSON list of files and folders in the specified directory.
    /// Includes security checks to prevent path traversal attacks.
    pub(crate) async fn browse(
        &self,
        query: &str,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let base_path = CONFIG
            .read_with(|cfg| cfg.canonical_base_path.clone())
            .await;
        let params = super::helpers::parse_query(query);
        let path = params.get("path").map(|s| s.as_str()).unwrap_or("");
        let full_path = if path.is_empty() {
            base_path.clone()
        } else {
            // --------------------------------------------------
            // safe_join canonicalizes and verifies containment, blocking both
            // `../` traversal and symlink escapes
            // --------------------------------------------------
            match self.safe_join(path).await {
                Some(p) => p,
                None => {
                    tracing::warn!("Browse: path traversal/symlink escape blocked for '{path}'");
                    return Ok(HttpResponse::forbidden());
                }
            }
        };
        // --------------------------------------------------
        // getting path, acc in items Vec
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
        // get entries of the selected dir (blocking I/O on thread pool)
        // --------------------------------------------------
        let base_path_owned = base_path.clone();
        let full_path_clone = full_path.clone();
        match async {
            // --------------------------------------------------
            // get entries of the selected dir (non-blocking using smol)
            // --------------------------------------------------
            let mut result = Vec::with_capacity(NUM_FILES_PRE_ALLOCATION);
            let mut entries = smol::fs::read_dir(&full_path_clone).await?;
            while let Some(entry) = entries.try_next().await? {
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
                result.push(FileItem {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: relative_path,
                    is_dir,
                    size,
                });
            }
            Ok::<_, std::io::Error>(result)
        }
        .await
        {
            // --------------------------------------------------
            // if entries are found, extend the items vector
            // --------------------------------------------------
            Ok(entries) => {
                tracing::debug!("Successfully read directory: {full_path:?}");
                items.extend(entries);
            }
            // --------------------------------------------------
            // otherwise, return a 404 response
            // --------------------------------------------------
            Err(e) => {
                tracing::error!("Failed to read directory {full_path:?}: {e}");
                return Ok(HttpResponse::not_found());
            }
        };
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
        HttpResponse::ok().body_json(&items).map_err(Into::into)
    }
}
