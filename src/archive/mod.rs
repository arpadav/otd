//! Archive creation and caching for multi-file downloads
//!
//! Defines the [`ArchiveWriter`] trait, the [`compute_cache_key`] function for
//! content-addressed caching, and [`spawn_archive_creation`] which drives
//! background archive building for active multi-file links
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod file;
mod formats;
mod writers;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use formats::CompressionType;
pub(crate) use {file::CompressedFile, writers::OmniWriter};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::state::ArchiveState;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(feature = "parallel")]
/// Entry state for hashing and caching
type EntryState = Option<(String, u64)>;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Cache directory for pre-built archives
pub(crate) const ARCHIVE_CACHE_DIR: &str = "/tmp/otd-cache";
/// Default directory name when a path has no filename component
pub(crate) const DEFAULT_DIR_NAME: &str = "folder";
/// Default filename when a path has no filename component
pub(crate) const DEFAULT_FILE_NAME: &str = "file";
/// Default download name fallback for single items
pub(crate) const DEFAULT_DOWNLOAD_NAME: &str = "download";
/// Unix permissions applied to entries inside archives
const ARCHIVE_UNIX_PERMISSIONS: u32 = 0o755;

/// Trait for format-agnostic archive writing
///
/// Implementors must support appending files and directories by archive path,
/// and must produce a valid, finalized archive when [`finish`][Self::finish]
/// is called. Entries are always written eagerly into memory before being
/// passed here; streaming is not supported because tar archives require the
/// file size in the header before writing content
trait ArchiveWriter<W: Write + std::io::Seek> {
    /// Appends a file entry to the archive
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path the file will have inside the archive
    /// * `contents` - The raw bytes of the file to append
    fn add_file(&mut self, archive_path: &str, contents: &[u8]) -> std::io::Result<()>;

    /// Appends a directory entry to the archive
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path the directory will have inside the archive
    fn add_directory(&mut self, archive_path: &str) -> std::io::Result<()>;

    /// Finalizes the archive and returns the underlying writer
    fn finish(self) -> std::io::Result<W>;
}

/// Computes a deterministic content-addressed cache key from a set of paths
///
/// Walks directories recursively, collects `(canonical_path, mtime_nanos)` for
/// every file encountered, inserts them into a [`BTreeSet`] for deterministic
/// ordering, then hashes the sorted entries with [`std::hash::DefaultHasher`]
/// and returns a 16-character lowercase hex string
///
/// When the `parallel` feature is enabled, directory traversal uses `jwalk`
/// for parallel I/O; otherwise `walkdir` is used sequentially
///
/// # Arguments
///
/// * `paths` - Slice of file or directory paths to include in the key
///
/// # Safety
///
/// Uses `std::hash::DefaultHasher` which is not cryptographically secure
/// This is intentional - the key is used only for cache lookups, not security
fn compute_cache_key(paths: &[PathBuf]) -> std::io::Result<String> {
    /// Resolves canonical path and modification time (nanoseconds since epoch)
    /// for a single file, used to produce a stable cache key contribution
    fn fetch_entry_metadata(path: &Path) -> std::io::Result<(String, u64)> {
        let canonical = std::fs::canonicalize(path)?;
        let mtime = canonical
            .metadata()?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Ok((canonical.to_string_lossy().into_owned(), mtime))
    }

    // --------------------------------------------------
    // flat map traversal, into a BTreeSet for sorting
    // --------------------------------------------------
    #[cfg(feature = "parallel")]
    let entries: BTreeSet<(String, u64)> = {
        // --------------------------------------------------
        // use jwalk for parallel traversal
        // --------------------------------------------------
        paths
            .iter()
            .flat_map(|path| {
                if path.is_dir() {
                    itertools::Either::Left(
                        jwalk::WalkDirGeneric::<((), EntryState)>::new(path)
                            .process_read_dir(|_depth, _path, _state, children| {
                                children.iter_mut().for_each(|entry| {
                                    if let Ok(entry) = entry
                                        && entry.file_type.is_file()
                                    {
                                        entry.client_state =
                                            fetch_entry_metadata(&entry.path()).ok();
                                    }
                                });
                            })
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter_map(|e| e.client_state),
                    )
                } else {
                    itertools::Either::Right(fetch_entry_metadata(path).into_iter())
                }
            })
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let entries: BTreeSet<(String, u64)> = {
        // --------------------------------------------------
        // seq walk and fetch metadata
        // --------------------------------------------------
        paths
            .iter()
            .flat_map(|path| {
                if path.is_dir() {
                    itertools::Either::Left(
                        walkdir::WalkDir::new(path)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| e.file_type().is_file())
                            .map(|e| e.into_path()),
                    )
                } else {
                    itertools::Either::Right(std::iter::once(path.clone()))
                }
            })
            .map(|p| fetch_entry_metadata(&p))
            .collect::<Result<_, _>>()?
    };

    // --------------------------------------------------
    // compute cache key from sorted entries
    // --------------------------------------------------
    let mut hasher = std::hash::DefaultHasher::new();
    for (path, mtime) in &entries {
        path.hash(&mut hasher);
        mtime.hash(&mut hasher);
    }
    // --------------------------------------------------
    // return hex-encoded cache key
    // --------------------------------------------------
    Ok(format!("{:016x}", hasher.finish()))
}

/// Spawns a background task that builds and caches the archive for a multi-file link
///
/// Clones the global app state, then spawns a tokio task which offloads the
/// blocking archive construction to [`tokio::task::spawn_blocking`]. On success,
/// the token's [`ArchiveState`] is updated to [`ArchiveState::Ready`] with the
/// cache path. On failure, it is set to [`ArchiveState::Failed`] with the error
/// message. If the computed cache file already exists on disk, the write is
/// skipped (cache hit)
///
/// # Arguments
///
/// * `token` - The download token this archive belongs to
/// * `paths` - Source file/directory paths to include in the archive
/// * `compression` - The archive format to produce
pub(crate) fn spawn_archive_creation(
    token: String,
    paths: Vec<PathBuf>,
    compression: CompressionType,
) {
    // --------------------------------------------------
    // clone the app state handle for use inside the task
    // --------------------------------------------------
    let state = crate::APP_STATE.clone();
    tokio::spawn(async move {
        let token_inner = token.clone();
        // --------------------------------------------------
        // offload blocking archive I/O to a thread pool task
        // --------------------------------------------------
        let result: Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> =
            tokio::task::spawn_blocking(move || {
                // --------------------------------------------------
                // compute a content-addressed cache key from the
                // source paths and their modification times
                // --------------------------------------------------
                let hash = compute_cache_key(&paths)?;
                let ext = compression.extension();
                std::fs::create_dir_all(ARCHIVE_CACHE_DIR)?;
                let cache_path = PathBuf::from(format!("{ARCHIVE_CACHE_DIR}/{hash}{ext}"));
                // --------------------------------------------------
                // return early on cache hit - archive already built
                // --------------------------------------------------
                if cache_path.exists() {
                    tracing::info!("Archive cache hit for link {token_inner}: {cache_path:?}");
                    return Ok(cache_path);
                }
                // --------------------------------------------------
                // write to a .tmp path then atomically rename so
                // concurrent readers never see a partial archive
                // --------------------------------------------------
                let tmp_path = cache_path.with_extension(format!("{}.tmp", &ext[1..]));
                let compressed = CompressedFile::new(paths, compression);
                compressed.write_to_file(&tmp_path)?;
                std::fs::rename(&tmp_path, &cache_path)?;
                tracing::info!("Archive created for link {token_inner}: {cache_path:?}");
                Ok(cache_path)
            })
            .await
            .unwrap_or_else(|e| Err(e.into()));
        // --------------------------------------------------
        // update the token's archive state with the result
        // --------------------------------------------------
        let links = state.links.read().await;
        if let Some(item) = links.get(&token) {
            let mut zs = item.archive_state.write().await;
            match result {
                Ok(path) => *zs = ArchiveState::Ready(path),
                Err(e) => {
                    tracing::error!("Archive creation failed for link {token}: {e}");
                    *zs = ArchiveState::Failed(e.to_string());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: creates a temp dir with test files and returns its path
    fn create_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), b"hello world").unwrap();
        std::fs::write(dir.path().join("data.bin"), b"\x00\x01\x02\x03").unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.txt"), b"nested content").unwrap();
        dir
    }

    #[test]
    fn test_compression_type_extension() {
        assert_eq!(CompressionType::Zip.extension(), ".zip");
        assert_eq!(CompressionType::TarGz.extension(), ".tar.gz");
        assert_eq!(CompressionType::Tar.extension(), ".tar");
    }

    #[test]
    fn test_compression_type_content_type() {
        assert_eq!(CompressionType::Zip.content_type(), "application/zip");
        assert_eq!(CompressionType::TarGz.content_type(), "application/gzip");
        assert_eq!(CompressionType::Tar.content_type(), "application/x-tar");
    }

    #[test]
    fn test_ensure_extension() {
        let cf = CompressedFile::new(Vec::new(), CompressionType::Zip);
        assert_eq!(cf.ensure_extension("archive.zip").as_ref(), "archive.zip");
        assert_eq!(cf.ensure_extension("archive").as_ref(), "archive.zip");

        let cf = CompressedFile::new(Vec::new(), CompressionType::TarGz);
        assert_eq!(
            cf.ensure_extension("archive.tar.gz").as_ref(),
            "archive.tar.gz"
        );
        assert_eq!(cf.ensure_extension("archive").as_ref(), "archive.tar.gz");
    }

    #[test]
    fn test_archive_zip_roundtrip() {
        let dir = create_test_dir();
        let cf = CompressedFile::new(vec![dir.path().to_path_buf()], CompressionType::Zip);
        let data = cf.write_to_memory().unwrap();
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n.contains("hello.txt")));
        assert!(names.iter().any(|n| n.contains("nested.txt")));
    }

    #[test]
    fn test_archive_tar_roundtrip() {
        let dir = create_test_dir();
        let cf = CompressedFile::new(vec![dir.path().to_path_buf()], CompressionType::Tar);
        let data = cf.write_to_memory().unwrap();
        let mut archive = tar::Archive::new(std::io::Cursor::new(data));
        let names: Vec<String> = archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n.contains("hello.txt")));
        assert!(names.iter().any(|n| n.contains("nested.txt")));
    }

    #[test]
    fn test_archive_targz_roundtrip() {
        let dir = create_test_dir();
        let cf = CompressedFile::new(vec![dir.path().to_path_buf()], CompressionType::TarGz);
        let data = cf.write_to_memory().unwrap();
        let decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(data));
        let mut archive = tar::Archive::new(decoder);
        let names: Vec<String> = archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n.contains("hello.txt")));
        assert!(names.iter().any(|n| n.contains("nested.txt")));
    }
}
