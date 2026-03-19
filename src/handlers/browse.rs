//! File browsing handler for the admin interface.
//!
//! Author: aav
use crate::{http::*, types::*};

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
        let params = self.parse_query(query);
        let path = params.get("path").map(|s| s.as_str()).unwrap_or("");

        let full_path = if path.is_empty() {
            self.state.base_path.clone()
        } else {
            // safe_join canonicalizes and verifies containment, blocking both
            // `../` traversal and symlink escapes.
            match self.safe_join(path) {
                Some(p) => p,
                None => {
                    tracing::warn!("Browse: path traversal/symlink escape blocked for '{path}'");
                    return Ok(HttpResponse::forbidden());
                }
            }
        };

        tracing::info!("Browse request - path: '{path}', full_path: '{full_path:?}'");

        let mut items = Vec::new();

        // Add parent directory if not at root
        if full_path != self.state.base_path
            && let Some(parent) = full_path.parent()
            && parent.starts_with(&self.state.base_path)
        {
            let relative_parent = parent
                .strip_prefix(&self.state.base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            items.push(FileItem {
                name: "..".to_string(),
                path: relative_parent,
                is_dir: true,
                size: None,
            });
        }

        let entries = match std::fs::read_dir(&full_path) {
            Ok(entries) => {
                tracing::info!("Successfully read directory: {full_path:?}");
                entries
            }
            Err(e) => {
                tracing::error!("Failed to read directory {full_path:?}: {e}");
                return Ok(HttpResponse::not_found());
            }
        };

        for entry in entries.flatten() {
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = if is_dir {
                None
            } else {
                metadata.map(|m| m.len())
            };

            let entry_path = entry.path();
            let relative_path = entry_path
                .strip_prefix(&self.state.base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());

            items.push(FileItem {
                name: entry.file_name().to_string_lossy().to_string(),
                path: relative_path,
                is_dir,
                size,
            });
        }

        // Sort: directories first, then files, both alphabetically
        items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        HttpResponse::ok().body_json(&items).map_err(Into::into)
    }
}
