//! Download link generation and token management.
//!
//! Author: aav
use super::download::{DEFAULT_DIR_NAME, DEFAULT_DOWNLOAD_NAME};
use crate::{http::*, types::*};

use std::sync::{Arc, atomic::Ordering};
use uuid::Uuid;

/// Link management methods for [`super::Handler`].
impl super::Handler {
    /// Generates a new download link for the specified files.
    ///
    /// Creates a unique token and stores the download item in the application state.
    /// The generated URL includes the filename to help wget and browsers save files correctly.
    pub(crate) async fn generate_link(
        &self,
        body: &str,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let generate_req: GenerateRequest = serde_json::from_str(body)?;

        if generate_req.paths.is_empty() {
            return Ok(HttpResponse::bad_request().body_text("No paths provided"));
        }

        let mut full_paths = Vec::new();

        // Validate all paths - safe_join canonicalizes and checks containment,
        // blocking both `../` traversal and symlink-based escapes.
        for path_str in &generate_req.paths {
            match self.safe_join(path_str).await {
                Some(full_path) => full_paths.push(full_path),
                None => {
                    tracing::warn!(
                        "Generate: path traversal/symlink escape blocked for '{path_str}'"
                    );
                    return Ok(HttpResponse::forbidden());
                }
            }
        }

        let token = Uuid::new_v4().to_string();
        let is_multi_file =
            full_paths.len() > 1 || (full_paths.len() == 1 && full_paths[0].is_dir());

        let compression = generate_req.format;
        let ext = compression.extension();

        // Determine the name
        let name = if let Some(custom_name) = generate_req.name {
            custom_name
        } else if full_paths.len() == 1 {
            let path = &full_paths[0];
            if path.is_dir() {
                format!(
                    "{}{}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(DEFAULT_DIR_NAME),
                    ext
                )
            } else {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(DEFAULT_DOWNLOAD_NAME)
                    .to_string()
            }
        } else {
            format!("output{ext}")
        };

        let max_downloads = generate_req.max_downloads.unwrap_or(1).max(1);
        let expires_at = generate_req
            .expires_in_seconds
            .map(|secs| std::time::Instant::now() + std::time::Duration::from_secs(secs));

        let archive_state = if is_multi_file {
            smol::lock::RwLock::new(ArchiveState::Preparing)
        } else {
            smol::lock::RwLock::new(ArchiveState::NotNeeded)
        };

        let item = DownloadItem {
            paths: full_paths.clone(),
            is_multi_file,
            name: name.clone(),
            max_downloads,
            download_count: std::sync::atomic::AtomicU32::new(0),
            expires_at,
            created_at: std::time::Instant::now(),
            compression,
            archive_state,
        };

        self.state.tokens.write().await.insert(token.clone(), item);
        self.state.mark_dirty();

        // Spawn background archive creation for multi-file downloads
        if is_multi_file {
            super::Handler::spawn_archive_creation(
                Arc::clone(&self.state),
                token.clone(),
                full_paths,
                compression,
            );
        }

        // Create URL with filename for better wget/browser behavior
        let download_url = self.download_url(&name, &token).await;
        tracing::info!("Generated download link for '{name}': {token}");
        let response = GenerateResponse {
            token,
            download_url,
        };

        HttpResponse::ok().body_json(&response).map_err(Into::into)
    }

    /// Lists all active download tokens with their status.
    pub(crate) async fn list_tokens(
        &self,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let tokens = self.state.tokens.read().await;
        let mut items = Vec::with_capacity(tokens.len());

        let now = std::time::Instant::now();
        for (token, item) in tokens.iter() {
            let download_url = self.download_url(&item.name, token).await;
            let count = item.download_count.load(Ordering::Relaxed);
            let expired = item.expires_at.map(|e| now >= e).unwrap_or(false);
            let expires_in_seconds = item.expires_at.and_then(|e| {
                if now < e {
                    Some(e.duration_since(now).as_secs())
                } else {
                    None
                }
            });
            let archive_status = {
                let state = item.archive_state.read().await;
                match &*state {
                    ArchiveState::NotNeeded => "not_needed",
                    ArchiveState::Preparing => "preparing",
                    ArchiveState::Ready(_) => "ready",
                    ArchiveState::Failed(_) => "failed",
                }
            };
            items.push(TokenListItem {
                token: token.clone(),
                name: item.name.clone(),
                is_multi_file: item.is_multi_file,
                download_count: count,
                max_downloads: item.max_downloads,
                remaining_downloads: item.max_downloads.saturating_sub(count),
                expired,
                expires_in_seconds,
                download_url,
                paths: item
                    .paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                archive_status: archive_status.to_string(),
            });
        }

        HttpResponse::ok().body_json(&items).map_err(Into::into)
    }

    /// Deletes tokens matching a filter: "used", "expired", or "all".
    pub(crate) async fn bulk_delete_tokens(
        &self,
        body: &str,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(serde::Deserialize)]
        struct BulkDeleteRequest {
            filter: String,
        }

        let req: BulkDeleteRequest = serde_json::from_str(body)?;
        let mut tokens = self.state.tokens.write().await;
        let before = tokens.len();
        let now = std::time::Instant::now();

        tokens.retain(|_, item| {
            let count = item.download_count.load(Ordering::Relaxed);
            let is_expired = item.expires_at.is_some_and(|e| now >= e);
            let is_used = count >= item.max_downloads;
            match req.filter.as_str() {
                "used" => !is_used,
                "expired" => !is_expired,
                "all" => false,
                _ => true,
            }
        });

        let removed = before - tokens.len();
        if removed > 0 {
            self.state.mark_dirty();
        }
        let filter = &req.filter;
        tracing::info!("Bulk delete (filter={filter}): removed {removed} tokens");

        HttpResponse::ok()
            .body_json(&BulkDeleteResponse { removed })
            .map_err(Into::into)
    }

    /// Deletes a download token.
    pub(crate) async fn delete_token(
        &self,
        token: &str,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut tokens = self.state.tokens.write().await;
        if tokens.remove(token).is_some() {
            self.state.mark_dirty();
            tracing::info!("Deleted token: {token}");
            HttpResponse::ok()
                .body_json(&BulkDeleteResponse { removed: 1 })
                .map_err(Into::into)
        } else {
            HttpResponse::ok()
                .body_json(&BulkDeleteResponse { removed: 0 })
                .map_err(Into::into)
        }
    }
}
