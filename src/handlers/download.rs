//! Download serving and archive creation.
//!
//! This module handles file downloads (single and multi-file) and all
//! archive operations including background creation and caching.
//! Archive creation is abstracted behind [`CompressedFile`] to support
//! multiple compression formats via [`CompressionType`].
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{http::*, types::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};
use zip::write::FileOptions;

/// Cache directory for pre-built archives.
pub(crate) const ARCHIVE_CACHE_DIR: &str = "/tmp/otd-cache";
/// Default directory name when a path has no filename component.
pub(crate) const DEFAULT_DIR_NAME: &str = "folder";
/// Default filename when a path has no filename component.
pub(crate) const DEFAULT_FILE_NAME: &str = "file";
/// Default download name fallback for single items.
pub(crate) const DEFAULT_DOWNLOAD_NAME: &str = "download";
/// Unix permissions applied to entries inside archives.
const ARCHIVE_UNIX_PERMISSIONS: u32 = 0o755;

/// Entry state - for hashing + caching :)
type EntryState = Option<(String, u64)>;

/// Supported archive/compression formats.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionType {
    /// Standard ZIP archive (deflate).
    #[default]
    Zip,
    /// Gzip-compressed tar archive.
    #[serde(alias = "targz", alias = "tgz")]
    TarGz,
    /// Uncompressed tar archive.
    Tar,
}
/// [`CompressionType`] implementation
impl CompressionType {
    /// File extension for this compression type (including the leading dot).
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Zip => ".zip",
            Self::TarGz => ".tar.gz",
            Self::Tar => ".tar",
        }
    }

    /// HTTP `Content-Type` header value for this format.
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Zip => content_type::ZIP,
            Self::TarGz => content_type::GZIP,
            Self::Tar => content_type::TAR,
        }
    }
}
/// Tar archive operations shared by both compressed and uncompressed variants.
struct TarWriter<W: Write>(tar::Builder<W>);
/// [`TarWriter`] implementation
impl<W: Write> TarWriter<W> {
    fn add_file(
        &mut self,
        archive_path: &str,
        contents: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(ARCHIVE_UNIX_PERMISSIONS);
        header.set_cksum();
        self.0.append_data(&mut header, archive_path, contents)?;
        Ok(())
    }

    fn add_directory(
        &mut self,
        archive_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(ARCHIVE_UNIX_PERMISSIONS);
        header.set_entry_type(tar::EntryType::Directory);
        header.set_cksum();
        let dir_path = if archive_path.ends_with('/') {
            std::borrow::Cow::Borrowed(archive_path)
        } else {
            std::borrow::Cow::Owned(format!("{archive_path}/"))
        };
        self.0
            .append_data(&mut header, dir_path.as_ref(), &[][..])?;
        Ok(())
    }

    fn finish(self) -> Result<W, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.0.into_inner()?)
    }
}

/// Wraps a format-specific archive writer so callers don't need to know
/// which compression backend is in use.
enum ArchiveWriter<W: Write + std::io::Seek> {
    Zip(Box<zip::ZipWriter<W>>),
    TarGz(Box<TarWriter<flate2::write::GzEncoder<W>>>),
    Tar(Box<TarWriter<W>>),
}
/// [`ArchiveWriter`] implementation
impl<W: Write + std::io::Seek> ArchiveWriter<W> {
    /// Creates a new archive writer for the given compression type.
    fn new(compression: CompressionType, dest: W) -> Self {
        match compression {
            CompressionType::Zip => Self::Zip(Box::new(zip::ZipWriter::new(dest))),
            CompressionType::TarGz => {
                let encoder = flate2::write::GzEncoder::new(dest, flate2::Compression::default());
                Self::TarGz(Box::new(TarWriter(tar::Builder::new(encoder))))
            }
            CompressionType::Tar => Self::Tar(Box::new(TarWriter(tar::Builder::new(dest)))),
        }
    }

    /// Adds a single file to the archive under `archive_path`.
    fn add_file(
        &mut self,
        archive_path: &str,
        contents: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Zip(zip) => {
                let options = FileOptions::<()>::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(ARCHIVE_UNIX_PERMISSIONS);
                zip.start_file(archive_path, options)?;
                zip.write_all(contents)?;
            }
            Self::TarGz(tar) => tar.add_file(archive_path, contents)?,
            Self::Tar(tar) => tar.add_file(archive_path, contents)?,
        }
        Ok(())
    }

    /// Adds an empty directory entry to the archive.
    fn add_directory(
        &mut self,
        archive_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Zip(zip) => {
                let options = FileOptions::<()>::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(ARCHIVE_UNIX_PERMISSIONS);
                zip.add_directory(archive_path, options)?;
            }
            Self::TarGz(tar) => tar.add_directory(archive_path)?,
            Self::Tar(tar) => tar.add_directory(archive_path)?,
        }
        Ok(())
    }

    /// Finalizes the archive. Must be called before the output is read.
    fn finish(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Self::Zip(zip) => {
                zip.finish()?;
            }
            Self::TarGz(tar) => {
                tar.finish()?.finish()?;
            }
            Self::Tar(tar) => {
                tar.finish()?;
            }
        }
        Ok(())
    }
}

/// A multi-file compressed archive that can be written to disk or memory.
///
/// `CompressedFile` owns the list of source paths and the target compression
/// format. Its methods create the archive through [`ArchiveWriter`], which
/// dispatches to the correct backend based on [`CompressionType`].
pub(crate) struct CompressedFile {
    /// Source file/directory paths to include in the archive.
    paths: Vec<PathBuf>,
    /// Which compression format to produce.
    compression: CompressionType,
}
/// [`CompressedFile`] implementation
impl CompressedFile {
    /// Creates a new `CompressedFile` for the given paths and format.
    pub(crate) fn new(paths: Vec<PathBuf>, compression: CompressionType) -> Self {
        Self { paths, compression }
    }

    /// Writes the archive to `output` on disk.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use otd::handlers::download::{CompressedFile, CompressionType};
    ///
    /// let paths = vec![PathBuf::from("file1.txt"), PathBuf::from("dir1")];
    /// let compressed = CompressedFile::new(paths, CompressionType::Zip);
    /// compressed.write_to_file(&PathBuf::from("output.zip")).unwrap();
    /// ```
    pub(crate) fn write_to_file(
        &self,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file = std::fs::File::create(output)?;
        let mut writer = ArchiveWriter::new(self.compression, file);
        self.add_paths(&mut writer)?;
        writer.finish()
    }

    /// Builds the archive in memory and returns the raw bytes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use otd::handlers::download::{CompressedFile, CompressionType};
    ///
    /// let paths = vec![PathBuf::from("file1.txt"), PathBuf::from("dir1")];
    /// let compressed = CompressedFile::new(paths, CompressionType::Zip);
    /// let bytes = compressed.write_to_memory().unwrap();
    /// ```
    pub(crate) fn write_to_memory(
        &self,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = ArchiveWriter::new(self.compression, cursor);
            self.add_paths(&mut writer)?;
            writer.finish()?;
        }
        Ok(buf)
    }

    /// Ensures the archive name has the correct extension for this format.
    pub(crate) fn ensure_extension<'a>(&self, name: &'a str) -> std::borrow::Cow<'a, str> {
        let ext = self.compression.extension();
        if name.ends_with(ext) {
            std::borrow::Cow::Borrowed(name)
        } else {
            std::borrow::Cow::Owned(format!("{name}{ext}"))
        }
    }

    /// Walks all source paths and feeds files/directories into the writer.
    fn add_paths<W: Write + std::io::Seek>(
        &self,
        writer: &mut ArchiveWriter<W>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.paths.iter().try_for_each(|path| {
            if path.is_dir() {
                self.add_directory(writer, path)
            } else {
                self.add_file(writer, path)
            }
        })
    }

    /// Recursively adds a directory and its contents to the archive
    fn add_directory<W: Write + std::io::Seek>(
        &self,
        writer: &mut ArchiveWriter<W>,
        dir_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // --------------------------------------------------
        // get dirname
        // --------------------------------------------------
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_DIR_NAME);
        // --------------------------------------------------
        // get dirname
        // --------------------------------------------------
        for entry in walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            // --------------------------------------------------
            // strip prefix to get relative path
            // --------------------------------------------------
            let relative_path = match entry_path.strip_prefix(dir_path) {
                Ok(path) if path.as_os_str().is_empty() => continue,
                Ok(path) => path,
                Err(_) => continue,
            };
            let archive_path = format!("{}/{}", dir_name, relative_path.to_string_lossy());
            // --------------------------------------------------
            // add entry to archive for zipping / tarballing
            // --------------------------------------------------
            if entry.file_type().is_dir() {
                // --------------------------------------------------
                // if dir
                // --------------------------------------------------
                if let Err(e) = writer.add_directory(&archive_path) {
                    tracing::error!("Failed to add directory to archive: {e}");
                }
            } else {
                // --------------------------------------------------
                // if file
                // --------------------------------------------------
                let file_contents = match std::fs::read(entry_path) {
                    Ok(contents) => contents,
                    Err(e) => {
                        tracing::error!("Failed to read file for archive: {e}");
                        continue;
                    }
                };
                if let Err(e) = writer.add_file(&archive_path, &file_contents) {
                    tracing::error!("Failed to write file to archive: {e}");
                }
            }
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(())
    }

    /// Adds a single top-level file to the archive
    fn add_file<W: Write + std::io::Seek>(
        &self,
        writer: &mut ArchiveWriter<W>,
        file_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_FILE_NAME);
        let file_contents = std::fs::read(file_path)?;
        writer.add_file(filename, &file_contents)
    }
}

/// Computes a deterministic cache key from a set of paths.
///
/// Walks directories recursively, collects (canonical_path, mtime) for every file,
/// sorts them, and produces a hex-encoded hash.
fn compute_cache_key(paths: &[PathBuf]) -> std::io::Result<String> {
    /// Fetches the canonical path and mtime of a file - helper function in
    /// order to compute the cache key deterministically.
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
        if #[cfg(feature = "jwalk")] {
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
/// [`Handler`] implementation
impl super::Handler {
    /// Handles file download requests using the provided token.
    ///
    /// Validates the token, checks one-time usage rules, and serves the file(s).
    /// Supports both single file downloads and multi-file archive downloads.
    pub(crate) async fn download(
        &self,
        query: &str,
        peer_addr: std::net::SocketAddr,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        // --------------------------------------------------
        // parse http query parameters
        // --------------------------------------------------
        let params = super::helpers::parse_query(query);
        // --------------------------------------------------
        // get the token
        // --------------------------------------------------
        let token = match params.get("k") {
            Some(token) => token,
            None => {
                tracing::info!("Download from {peer_addr}: missing token parameter");
                return Ok(HttpResponse::bad_request().body_text("Missing token parameter"));
            }
        };
        let tokens = self.state.tokens.read().await;
        let item = match tokens.get(token) {
            Some(item) => item,
            None => {
                tracing::info!("Download from {peer_addr}: token not found: {token}");
                return Ok(HttpResponse::not_found().body_text("Token not found"));
            }
        };
        // --------------------------------------------------
        // check if expired
        // --------------------------------------------------
        if let Some(expires_at) = item.expires_at
            && std::time::Instant::now() >= expires_at
        {
            tracing::info!("Download from {peer_addr}: token expired: {token}");
            return Ok(HttpResponse::gone().body_text("Download link has expired"));
        }
        // --------------------------------------------------
        // check download count against per-link max_downloads limit
        // reserve a slot via compare-exchange (don't commit yet)
        // --------------------------------------------------
        let mut current = item.download_count.load(Ordering::Acquire);
        loop {
            // --------------------------------------------------
            // early return on max downloads
            // --------------------------------------------------
            if current >= item.max_downloads {
                let max = item.max_downloads;
                tracing::info!("Download from {peer_addr}: limit reached for {token} (max {max})");
                return Ok(HttpResponse::gone().body_text("Download limit reached"));
            }
            // --------------------------------------------------
            // atomically increment download count
            // --------------------------------------------------
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
        // build the response - download count is tentatively
        // incremented. active_serving guards the cache file
        // from deletion while the file read is in progress.
        // --------------------------------------------------
        let response = if item.is_multi_file || (item.paths.len() == 1 && item.paths[0].is_dir()) {
            // --------------------------------------------------
            // if multifile, then read the archive state and extract what
            // we need without holding the guard across .await
            // --------------------------------------------------
            let state = item.archive_state.read().await;
            let name = item.name.clone();
            let compression = item.compression;
            match &*state {
                // --------------------------------------------------
                // archive still being prepared
                // --------------------------------------------------
                ArchiveState::Preparing => {
                    tracing::debug!(
                        "Download from {peer_addr}: archive still preparing for {token}"
                    );
                    Err(HttpResponse::accepted()
                        .content_type(content_type::PLAIN_TEXT)
                        .body_text("File is being prepared, please try again shortly"))
                }
                // --------------------------------------------------
                // serve the cached archive
                // --------------------------------------------------
                ArchiveState::Ready(cache_path) => {
                    tracing::info!(
                        "Download from {peer_addr}: serving archive '{name}' for {token}"
                    );
                    item.active_serving.fetch_add(1, Ordering::AcqRel);
                    let result = self
                        .serve_cached_archive(cache_path, &name, compression)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to serve archive for {token}: {e}");
                            HttpResponse::internal_server_error()
                                .body_text("Failed to serve archive")
                        });
                    item.active_serving.fetch_sub(1, Ordering::AcqRel);
                    result
                }
                // --------------------------------------------------
                // failed to create the archive
                // --------------------------------------------------
                ArchiveState::Failed(err) => Err(HttpResponse::internal_server_error()
                    .body_text(&format!("Archive creation failed: {err}"))),
                // --------------------------------------------------
                // fallback: should not happen
                // --------------------------------------------------
                ArchiveState::NotNeeded => {
                    item.active_serving.fetch_add(1, Ordering::AcqRel);
                    let result = self
                        .serve_as_archive(&item.paths, &name, compression)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to serve archive for {token}: {e}");
                            HttpResponse::internal_server_error()
                                .body_text("Failed to serve archive")
                        });
                    item.active_serving.fetch_sub(1, Ordering::AcqRel);
                    result
                }
            }
        } else {
            // --------------------------------------------------
            // serve a single file - no archive needed
            // --------------------------------------------------
            let name = &item.name;
            tracing::info!("Download from {peer_addr}: serving '{name}' for {token}");
            item.active_serving.fetch_add(1, Ordering::AcqRel);
            let result = self.serve_single_file(&item.paths[0]).await.map_err(|e| {
                tracing::error!("Failed to serve file for {token}: {e}");
                HttpResponse::internal_server_error().body_text("Failed to serve file")
            });
            item.active_serving.fetch_sub(1, Ordering::AcqRel);
            result
        };
        // --------------------------------------------------
        // check response - this affects the download count
        // --------------------------------------------------
        match response {
            Ok(resp) => {
                // --------------------------------------------------
                // success - count stays incremented, mark dirty
                // --------------------------------------------------
                self.state.mark_dirty();
                Ok(resp)
            }
            Err(error_resp) => {
                // --------------------------------------------------
                // failed - undo the increment so it doesn't count
                // --------------------------------------------------
                item.download_count.fetch_sub(1, Ordering::AcqRel);
                Ok(error_resp)
            }
        }
    }

    /// Serves a single file with appropriate headers for download.
    ///
    /// Helper function - used by [`Handler::download`] when item is not multi-file, so no zip/archive is needed.
    async fn serve_single_file(
        &self,
        path: &PathBuf,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let path_owned = path.clone();
        let file_contents = match smol::unblock(move || std::fs::read(&path_owned)).await {
            Ok(contents) => contents,
            Err(e) => {
                tracing::error!("File read error: {path:?} - {e}");
                return Ok(HttpResponse::not_found().body_text("File not found"));
            }
        };
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_FILE_NAME);
        // --------------------------------------------------
        // return the file
        // --------------------------------------------------
        Ok(HttpResponse::ok()
            .content_type(content_type::OCTET_STREAM)
            .attachment(filename)
            .body_bytes(file_contents))
    }

    /// Serves a previously cached archive from disk.
    ///
    /// Helper function - used by [`Handler::download`] when [`ArchiveState::Ready`] is encountered.
    async fn serve_cached_archive(
        &self,
        cache_path: &Path,
        name: &str,
        compression: CompressionType,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let cache_path_owned = cache_path.to_path_buf();
        let file_contents = match smol::unblock(move || std::fs::read(&cache_path_owned)).await {
            Ok(contents) => contents,
            Err(e) => {
                tracing::error!("Failed to read cached archive {cache_path:?}: {e}");
                return Ok(
                    HttpResponse::internal_server_error().body_text("Failed to read cached file")
                );
            }
        };
        let compressed = CompressedFile::new(Vec::new(), compression);
        let final_name = compressed.ensure_extension(name);
        // --------------------------------------------------
        // return the cached archive
        // --------------------------------------------------
        Ok(HttpResponse::ok()
            .content_type(compression.content_type())
            .attachment(&final_name)
            .body_bytes(file_contents))
    }

    /// Builds an archive in memory and serves it.
    ///
    /// Helper function - used by [`Handler::download`] when [`ArchiveState::NotNeeded`] is encountered,
    /// because the file is not an archive and needs to be compressed on the fly.
    ///
    /// This should theoretically never happen, as [`ArchiveState::NotNeeded`] is only used for non-archive files.
    async fn serve_as_archive(
        &self,
        paths: &[PathBuf],
        name: &str,
        compression: CompressionType,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        tracing::trace!(
            "Entered `serve_as_archive` (this shouldn't have happened!) for name: {name}"
        );
        let compressed = CompressedFile::new(paths.to_vec(), compression);
        let data = match compressed.write_to_memory() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to create archive: {e}");
                return Ok(
                    HttpResponse::internal_server_error().body_text("Failed to create archive")
                );
            }
        };
        let final_name = compressed.ensure_extension(name);
        // --------------------------------------------------
        // return the compressed data
        // --------------------------------------------------
        Ok(HttpResponse::ok()
            .content_type(compression.content_type())
            .attachment(&final_name)
            .body_bytes(data))
    }

    /// Spawns background archive creation for a multi-file download token.
    pub(crate) fn spawn_archive_creation(
        state: Arc<AppState>,
        token: String,
        paths: Vec<PathBuf>,
        compression: CompressionType,
    ) {
        smol::spawn(async move {
            let token_inner = token.clone();
            let result: Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> =
                smol::unblock(move || {
                    let hash = compute_cache_key(&paths)?;
                    let ext = compression.extension();
                    std::fs::create_dir_all(ARCHIVE_CACHE_DIR)?;
                    let cache_path = PathBuf::from(format!("{ARCHIVE_CACHE_DIR}/{hash}{ext}"));
                    if cache_path.exists() {
                        tracing::info!("Archive cache hit for token {token_inner}: {cache_path:?}");
                        return Ok(cache_path);
                    }
                    let tmp_path = cache_path.with_extension(format!("{}.tmp", &ext[1..]));
                    let compressed = CompressedFile::new(paths, compression);
                    compressed.write_to_file(&tmp_path)?;
                    std::fs::rename(&tmp_path, &cache_path)?;
                    tracing::info!("Archive created for token {token_inner}: {cache_path:?}");
                    Ok(cache_path)
                })
                .await;
            let tokens = state.tokens.read().await;
            if let Some(item) = tokens.get(&token) {
                let mut zs = item.archive_state.write().await;
                match result {
                    Ok(path) => *zs = ArchiveState::Ready(path),
                    Err(e) => {
                        tracing::error!("Archive creation failed for token {token}: {e}");
                        *zs = ArchiveState::Failed(e.to_string());
                    }
                }
            }
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_extension() {
        assert_eq!(CompressionType::Zip.extension(), ".zip");
        assert_eq!(CompressionType::TarGz.extension(), ".tar.gz");
        assert_eq!(CompressionType::Tar.extension(), ".tar");
    }

    #[test]
    fn test_compression_type_content_type() {
        assert_eq!(CompressionType::Zip.content_type(), content_type::ZIP);
        assert_eq!(CompressionType::TarGz.content_type(), content_type::GZIP);
        assert_eq!(CompressionType::Tar.content_type(), content_type::TAR);
    }

    #[test]
    fn test_compression_type_default() {
        assert_eq!(CompressionType::default(), CompressionType::Zip);
    }

    #[test]
    fn test_compression_type_serde() {
        let zip: CompressionType = serde_json::from_str(r#""zip""#).unwrap();
        assert_eq!(zip, CompressionType::Zip);

        let targz: CompressionType = serde_json::from_str(r#""tar_gz""#).unwrap();
        assert_eq!(targz, CompressionType::TarGz);

        // aliases
        let targz2: CompressionType = serde_json::from_str(r#""targz""#).unwrap();
        assert_eq!(targz2, CompressionType::TarGz);

        let tgz: CompressionType = serde_json::from_str(r#""tgz""#).unwrap();
        assert_eq!(tgz, CompressionType::TarGz);

        let tar: CompressionType = serde_json::from_str(r#""tar""#).unwrap();
        assert_eq!(tar, CompressionType::Tar);
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

        let cf = CompressedFile::new(Vec::new(), CompressionType::Tar);
        assert_eq!(cf.ensure_extension("archive.tar").as_ref(), "archive.tar");
        assert_eq!(cf.ensure_extension("archive").as_ref(), "archive.tar");
    }

    /// Helper: creates a temp dir with test files and returns its path.
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
        assert!(names.iter().any(|n| n.contains("data.bin")));
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
        assert!(names.iter().any(|n| n.contains("data.bin")));
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
        assert!(names.iter().any(|n| n.contains("data.bin")));
        assert!(names.iter().any(|n| n.contains("nested.txt")));
    }

    #[test]
    fn test_archive_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("single.txt");
        std::fs::write(&file_path, b"single file content").unwrap();

        for compression in [
            CompressionType::Zip,
            CompressionType::TarGz,
            CompressionType::Tar,
        ] {
            let cf = CompressedFile::new(vec![file_path.clone()], compression);
            let data = cf.write_to_memory().unwrap();
            assert!(
                !data.is_empty(),
                "archive should not be empty for {compression:?}"
            );
        }
    }
}
