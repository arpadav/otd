//! File download handler -- plain axum, no frontend
//!
//! Serves files directly with `Content-Type` and `Content-Disposition` headers
//! Error cases return plaintext with appropriate HTTP status codes
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

/// Query parameters extracted from the download URL (`?k=<token>`)
#[derive(serde::Deserialize)]
pub(crate) struct DownloadParams {
    k: Option<String>,
}

/// Download error variants with HTTP status codes and plaintext messages
enum DownloadError {
    /// No `?k=` parameter or empty token
    MissingToken,
    /// Token does not match any active link
    NotFound,
    /// Link has passed its expiry time
    Expired,
    /// All permitted downloads have been consumed
    LimitReached,
    /// Multi-file archive is still being built
    Preparing,
    /// Background archive creation failed
    ArchiveFailed(String),
    /// File I/O or archive read error at serve time
    ServeFailed,
}

/// Successful file download -- wraps raw bytes with HTTP metadata
struct DownloadResponse {
    content_type: String,
    filename: String,
    bytes: Vec<u8>,
}

/// [`DownloadError`] implementation
impl DownloadError {
    /// Returns the HTTP status code and plaintext body for this error
    fn status_and_message(self) -> (StatusCode, String) {
        match self {
            Self::MissingToken => (StatusCode::BAD_REQUEST, "Missing token parameter".into()),
            Self::NotFound => (StatusCode::NOT_FOUND, "Link not found".into()),
            Self::Expired => (StatusCode::GONE, "Download link has expired".into()),
            Self::LimitReached => (StatusCode::GONE, "Download limit reached".into()),
            Self::Preparing => (
                StatusCode::ACCEPTED,
                "File is being prepared, please try again shortly".into(),
            ),
            Self::ArchiveFailed(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Archive creation failed: {e}"),
            ),
            Self::ServeFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serve file".into())
            }
        }
    }
}

/// [`DownloadError`] implementation of [`IntoResponse`]
impl IntoResponse for DownloadError {
    fn into_response(self) -> Response {
        let (status, body) = self.status_and_message();
        (status, body).into_response()
    }
}

/// [`DownloadResponse`] implementation
impl DownloadResponse {
    fn new(content_type: impl Into<String>, filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            content_type: content_type.into(),
            filename: filename.into(),
            bytes,
        }
    }
}

/// [`DownloadResponse`] implementation of [`IntoResponse`]
impl IntoResponse for DownloadResponse {
    fn into_response(self) -> Response {
        let headers = [
            (header::CONTENT_TYPE, self.content_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", self.filename),
            ),
        ];
        (StatusCode::OK, headers, self.bytes).into_response()
    }
}

/// [`DownloadItem`] implementation
impl DownloadItem {
    /// Serves a multi-file or directory download from the archive cache
    async fn serve_archive(&self, token: &str) -> Result<DownloadResponse, DownloadError> {
        let archive = self.archive_state.read().await;
        let name = self.name.clone();
        let compression = self.compression;

        match &*archive {
            // --------------------------------------------------
            // archive still being prepared
            // --------------------------------------------------
            ArchiveState::Preparing(_) => {
                tracing::debug!("Download: archive still preparing for {token}");
                Err(DownloadError::Preparing)
            }
            // --------------------------------------------------
            // serve the cached archive
            // --------------------------------------------------
            ArchiveState::Ready(cache_path) => {
                tracing::info!("Download: serving archive '{name}' for {token}");
                self.active_serving.fetch_add(1, Ordering::AcqRel);
                let result = tokio::fs::read(cache_path)
                    .await
                    .map(|bytes| {
                        let cf = CompressedFile::new(Vec::new(), compression);
                        let final_name = cf.ensure_extension(&name).into_owned();
                        DownloadResponse::new(compression.content_type(), final_name, bytes)
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to read cached archive: {e}");
                        DownloadError::ServeFailed
                    });
                self.active_serving.fetch_sub(1, Ordering::AcqRel);
                result
            }
            // --------------------------------------------------
            // archive creation failed
            // --------------------------------------------------
            ArchiveState::Failed(err) => Err(DownloadError::ArchiveFailed(err.clone())),
            // --------------------------------------------------
            // fallback: should not happen for multi-file
            // --------------------------------------------------
            ArchiveState::NotNeeded => {
                self.active_serving.fetch_add(1, Ordering::AcqRel);
                let cf = CompressedFile::new(self.paths.clone(), compression);
                let result = cf
                    .write_to_memory()
                    .map(|data| {
                        let final_name = cf.ensure_extension(&name).into_owned();
                        DownloadResponse::new(compression.content_type(), final_name, data)
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to create archive: {e}");
                        DownloadError::ServeFailed
                    });
                self.active_serving.fetch_sub(1, Ordering::AcqRel);
                result
            }
        }
    }

    /// Serves a single file directly (no archive needed)
    async fn serve_single_file(&self) -> Result<DownloadResponse, DownloadError> {
        let name = self.name.clone();
        self.active_serving.fetch_add(1, Ordering::AcqRel);
        let result = tokio::fs::read(&self.paths[0])
            .await
            .map(|bytes| DownloadResponse::new("application/octet-stream", name, bytes))
            .map_err(|e| {
                tracing::error!("File read error: {e}");
                DownloadError::ServeFailed
            });
        self.active_serving.fetch_sub(1, Ordering::AcqRel);
        result
    }
}

/// Axum GET handler for file downloads
///
/// Validates the token, checks expiry and download limits, then serves the
/// file bytes with `Content-Disposition: attachment`. On any failure, returns
/// a plaintext error with the appropriate HTTP status code
pub(crate) async fn download_handler(Query(params): Query<DownloadParams>) -> Response {
    let Some(token) = params.k.filter(|k| !k.is_empty()) else {
        return DownloadError::MissingToken.into_response();
    };

    let links = crate::APP_STATE.links.read().await;
    let Some(item) = links.get(&token) else {
        return DownloadError::NotFound.into_response();
    };

    // --------------------------------------------------
    // check if expired
    // --------------------------------------------------
    if let Some(expires_at) = item.expires_at
        && std::time::Instant::now() >= expires_at
    {
        return DownloadError::Expired.into_response();
    }

    // --------------------------------------------------
    // check download count — reserve a slot via CAS
    // --------------------------------------------------
    let mut current = item.download_count.load(Ordering::Acquire);
    loop {
        if current >= item.max_downloads {
            return DownloadError::LimitReached.into_response();
        }
        match item.download_count.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }

    // --------------------------------------------------
    // build the response — count is tentatively incremented.
    // active_serving guards the cache file from deletion
    // while the read is in progress.
    // --------------------------------------------------
    let response = if item.is_multi_file || (item.paths.len() == 1 && item.paths[0].is_dir()) {
        item.serve_archive(&token).await
    } else {
        item.serve_single_file().await
    };

    // --------------------------------------------------
    // check response — undo the count increment on failure
    // --------------------------------------------------
    match response {
        Ok(resp) => {
            crate::APP_STATE.mark_dirty();
            resp.into_response()
        }
        Err(err) => {
            item.download_count.fetch_sub(1, Ordering::AcqRel);
            err.into_response()
        }
    }
}
