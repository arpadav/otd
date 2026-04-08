//! Core business logic for OTD
//!
//! Author: aav

// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod archive;
pub(crate) mod auth;
pub(crate) mod browse;
#[cfg(feature = "server")]
pub(crate) mod download;
#[cfg(feature = "server")]
pub(crate) mod health;
pub(crate) mod links;

// --------------------------------------------------
// server prelude - one-line import for core modules
// --------------------------------------------------
/// Server-only imports shared across core modules
///
/// Each module that contains `#[get]`/`#[post]` server functions
/// pulls in server deps with a single line:
/// ```rust,ignore
/// #[cfg(feature = "server")]
/// use super::prelude::*;
/// ```
#[cfg(feature = "server")]
pub(crate) mod prelude {
    pub(crate) use crate::config::CONFIG;
    pub(crate) use crate::core::archive::CompressedFile;
    pub(crate) use crate::state::{ArchiveState, DownloadItem};
    pub(crate) use std::sync::atomic::Ordering;
}

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use std::path::PathBuf;

/// Removes a cache file from disk, logging any errors that occur
///
/// # Arguments
///
/// * `path` - Path to the cache file to remove
#[cfg(feature = "server")]
pub(crate) fn remove_cache_file(path: &PathBuf) {
    match std::fs::remove_file(path) {
        Ok(()) => tracing::debug!("Removed cache file {path:?}"),
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!("Failed to remove cache file {path:?}: {e}");
        }
        _ => (),
    }
}

/// URL-encodes a string for safe use in URLs
///
/// # Arguments
///
/// * `input` - The string to encode
#[cfg(feature = "server")]
pub(crate) fn url_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    input
        .bytes()
        .fold(String::with_capacity(input.len()), |mut acc, b| {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    acc.push(b as char);
                }
                b' ' => acc.push('+'),
                _ => {
                    acc.push('%');
                    acc.push(HEX[(b >> 4) as usize] as char);
                    acc.push(HEX[(b & 0x0F) as usize] as char);
                }
            }
            acc
        })
}

#[cfg(feature = "server")]
/// Safely joins `relative` onto `base_path` and verifies the resolved
/// path is still within `base_path` after canonicalization
///
/// # Arguments
///
/// * `base_path` - The trusted base directory
/// * `relative` - The relative path to join
pub(crate) fn safe_join(base_path: &std::path::Path, relative: &str) -> Option<PathBuf> {
    let joined = base_path.join(relative);
    std::fs::canonicalize(&joined)
        .inspect_err(|e| tracing::warn!("Failed to canonicalize '{relative}': {e}"))
        .ok()
        .filter(|c| {
            let safe = c.starts_with(base_path);
            if !safe {
                tracing::warn!(
                    "Path escape blocked: '{relative}' resolves to '{c:?}' outside base '{base_path:?}'"
                );
            }
            safe
        })
}

#[cfg(feature = "server")]
/// Builds a full download URL from a display name and token
///
/// # Arguments
///
/// * `download_base_url` - Base URL prefix (e.g. `http://0.0.0.0:15205`)
/// * `name` - Display filename (will be URL-encoded)
/// * `token` - Unique download token
pub(crate) fn download_url(download_base_url: &str, name: &str, token: &str) -> String {
    format!("{}/{}?k={}", download_base_url, url_encode(name), token)
}
