//! Embedded frontend static file serving
//!
//! Uses `rust-embed` to include the SvelteKit build output at compile time,
//! serving it as a single-page application with index.html fallback
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/build/"]
/// Embedded frontend assets compiled from the SvelteKit build output
///
/// All files under `frontend/build/` are baked into the binary at compile
/// time via [`rust_embed::Embed`]. Served at runtime by [`spa_handler`]
struct FrontendAssets;

/// Serves embedded frontend assets with SPA fallback to `index.html`
///
/// Strips the leading `/` from the URI path and attempts an exact asset
/// lookup in [`FrontendAssets`]. If a matching file is found, it is served
/// with an appropriate `Content-Type` derived via `mime_guess`. If no exact
/// match exists (empty path or unknown route), falls back to `index.html`
/// so the SvelteKit client-side router can handle navigation. Returns
/// `404 Frontend not found` only if `index.html` itself is missing from the
/// embedded assets, which indicates a misconfigured build
///
/// # Arguments
///
/// * `uri` - The request URI, used to derive the asset path to look up
pub async fn spa_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // --------------------------------------------------
    // try exact file match
    // --------------------------------------------------
    if !path.is_empty()
        && let Some(file) = FrontendAssets::get(path)
    {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response();
    }
    // --------------------------------------------------
    // spa fallback: serve index.html
    // --------------------------------------------------
    match FrontendAssets::get("index.html") {
        Some(index) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            index.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Frontend not found").into_response(),
    }
}
