//! Archive creation and caching for multi-file downloads
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[cfg(feature = "server")]
mod file;
mod formats;
#[cfg(feature = "server")]
mod writers;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use formats::CompressionType;
#[cfg(feature = "server")]
pub(crate) use {file::CompressedFile, writers::OmniWriter};

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use crate::state::ArchiveState;

// --------------------------------------------------
// external
// --------------------------------------------------
#[cfg(feature = "server")]
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(all(feature = "parallel", feature = "server"))]
/// Entry state for hashing and caching
type EntryState = Option<(String, u64)>;

// --------------------------------------------------
// constants
// --------------------------------------------------
#[cfg(feature = "server")]
/// Cache directory for pre-built archives
pub(crate) const ARCHIVE_CACHE_DIR: &str = "/tmp/otd-cache";
#[cfg(feature = "server")]
/// Default directory name when a path has no filename component
pub(crate) const DEFAULT_DIR_NAME: &str = "folder";
#[cfg(feature = "server")]
/// Default filename when a path has no filename component
pub(crate) const DEFAULT_FILE_NAME: &str = "file";
#[cfg(feature = "server")]
/// Default download name fallback for single items
pub(crate) const DEFAULT_DOWNLOAD_NAME: &str = "download";
#[cfg(feature = "server")]
/// Unix permissions applied to entries inside archives
const ARCHIVE_UNIX_PERMISSIONS: u32 = 0o755;

#[cfg(feature = "server")]
/// A trait to be a valid archive writer
///
/// Note that theoretically since W impl Write, then this
/// could also have a `add_file_stream` method, but tar archives
/// require size in header, so a metadata read is required. This isn't
/// an issue unless archives are reading GB worth of data. So can
/// always implement later and move to the stream-based approach - aav
trait ArchiveWriter<W: Write + std::io::Seek> {
    /// Adds a file to the archive
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path to the file in the archive
    /// * `contents` - The contents of the file
    fn add_file(&mut self, archive_path: &str, contents: &[u8]) -> std::io::Result<()>;

    /// Adds a directory to the archive
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path to the directory in the archive
    fn add_directory(&mut self, archive_path: &str) -> std::io::Result<()>;

    /// Finishes writing the archive
    fn finish(self) -> std::io::Result<W>;
}

#[cfg(feature = "server")]
/// Computes a deterministic cache key from a set of paths
///
/// Walks directories recursively, collects (canonical_path, mtime) for every file,
/// sorts them, and produces a hex-encoded hash
fn compute_cache_key(paths: &[PathBuf]) -> std::io::Result<String> {
    /// Fetches the canonical path and mtime of a file - helper function in
    /// order to compute the cache key deterministically
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
    cfg_if::cfg_if! {
        if #[cfg(feature = "parallel")] {
            // --------------------------------------------------
            // use jwalk for parallel traversal
            // --------------------------------------------------
            let entries: BTreeSet<(String, u64)> = paths
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
                                            entry.client_state = fetch_entry_metadata(&entry.path()).ok();
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
                .collect();
        } else {
            // --------------------------------------------------
            // seq walk and fetch metadata
            // --------------------------------------------------
            let entries: BTreeSet<(String, u64)> = paths
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
                .collect::<Result<_, _>>()?;
        }
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

#[cfg(feature = "server")]
/// Spawns background archive creation for a multi-file download token
pub(crate) fn spawn_archive_creation(
    token: String,
    paths: Vec<PathBuf>,
    compression: CompressionType,
) {
    let state = crate::APP_STATE.clone();
    tokio::spawn(async move {
        let token_inner = token.clone();
        let result: Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> =
            tokio::task::spawn_blocking(move || {
                let hash = compute_cache_key(&paths)?;
                let ext = compression.extension();
                std::fs::create_dir_all(ARCHIVE_CACHE_DIR)?;
                let cache_path = PathBuf::from(format!("{ARCHIVE_CACHE_DIR}/{hash}{ext}"));
                if cache_path.exists() {
                    tracing::info!("Archive cache hit for link {token_inner}: {cache_path:?}");
                    return Ok(cache_path);
                }
                let tmp_path = cache_path.with_extension(format!("{}.tmp", &ext[1..]));
                let compressed = CompressedFile::new(paths, compression);
                compressed.write_to_file(&tmp_path)?;
                std::fs::rename(&tmp_path, &cache_path)?;
                tracing::info!("Archive created for link {token_inner}: {cache_path:?}");
                Ok(cache_path)
            })
            .await
            .unwrap_or_else(|e| Err(e.into()));

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

#[cfg(feature = "server")]
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
