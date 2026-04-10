//! Generic archive and compressed file abstraction
//!
//! Provides [`CompressedFile`], which owns a list of source paths and a
//! target [`CompressionType`] and produces archives via [`OmniWriter`]
//! Supports writing to disk or to an in-memory buffer
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{ArchiveWriter, CompressionType, DEFAULT_DIR_NAME, DEFAULT_FILE_NAME, OmniWriter};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// A multi-file compressed archive that can be written to disk or memory
///
/// `CompressedFile` owns the list of source paths and the target compression
/// format. Its methods create the archive through [`OmniWriter`], which
/// dispatches to the correct backend based on [`CompressionType`]
pub(crate) struct CompressedFile {
    /// Source file/directory paths to include in the archive
    paths: Vec<PathBuf>,
    /// Which compression format to produce
    compression: CompressionType,
}
/// [`CompressedFile`] implementation
impl CompressedFile {
    #[inline(always)]
    /// Creates a new `CompressedFile` for the given paths and format
    pub(crate) fn new(paths: Vec<PathBuf>, compression: CompressionType) -> Self {
        Self { paths, compression }
    }

    /// Writes the archive to `output` on disk
    ///
    /// Creates the file at `output`, builds the archive by walking all source
    /// paths, and finalizes the writer. Returns the underlying [`std::fs::File`]
    ///
    /// # Arguments
    ///
    /// * `output` - Destination path on disk to write the archive to
    pub(crate) fn write_to_file(&self, output: &Path) -> std::io::Result<std::fs::File> {
        // --------------------------------------------------
        // create the destination file and wrap in a writer
        // --------------------------------------------------
        let file = std::fs::File::create(output)?;
        let mut writer = OmniWriter::new(self.compression, file);
        // --------------------------------------------------
        // walk all source paths and add them to the archive
        // --------------------------------------------------
        self.add_paths(&mut writer)?;
        // --------------------------------------------------
        // finalize the archive and flush any trailing bytes
        // --------------------------------------------------
        writer.finish()
    }

    /// Builds the archive in memory and returns the raw bytes
    ///
    /// Writes into a `Vec<u8>` via a `Cursor`, walks all source paths, and
    /// finalizes the writer. The buffer is returned after the writer is dropped
    ///
    /// # Arguments
    ///
    /// *(none - uses the paths and compression format set at construction)*
    pub(crate) fn write_to_memory(&self) -> std::io::Result<Vec<u8>> {
        // --------------------------------------------------
        // allocate an in-memory buffer and wrap in a cursor
        // --------------------------------------------------
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = OmniWriter::new(self.compression, cursor);
            // --------------------------------------------------
            // walk all source paths and add them to the archive
            // --------------------------------------------------
            self.add_paths(&mut writer)?;
            // --------------------------------------------------
            // finalize the archive before dropping the cursor
            // --------------------------------------------------
            writer.finish()?;
        }
        Ok(buf)
    }

    #[inline(always)]
    /// Ensures the archive name has the correct extension for this compression format
    ///
    /// Returns the name unchanged if it already ends with the correct extension
    /// Otherwise appends the extension and returns an owned `String` wrapped in
    /// [`std::borrow::Cow::Owned`]
    ///
    /// # Arguments
    ///
    /// * `name` - The archive filename to check and potentially fix
    pub(crate) fn ensure_extension<'a>(&self, name: &'a str) -> std::borrow::Cow<'a, str> {
        let ext = self.compression.extension();
        if name.ends_with(ext) {
            std::borrow::Cow::Borrowed(name)
        } else {
            std::borrow::Cow::Owned(format!("{name}{ext}"))
        }
    }

    /// Walks all source paths and feeds files and directories into the writer
    ///
    /// Dispatches to [`add_directory`][Self::add_directory] for directories
    /// and [`add_file`][Self::add_file] for regular files. Propagates any
    /// I/O errors immediately via `?`
    ///
    /// # Arguments
    ///
    /// * `writer` - The archive writer to feed entries into
    fn add_paths<W: Write + std::io::Seek>(
        &self,
        writer: &mut OmniWriter<W>,
    ) -> std::io::Result<()> {
        self.paths.iter().try_for_each(|path| {
            if path.is_dir() {
                self.add_directory(writer, path)
            } else {
                self.add_file(writer, path)
            }
        })
    }

    /// Recursively adds a directory and all of its contents to the archive
    ///
    /// Walks the directory with `walkdir` (sequential) and adds each entry
    /// under a path prefixed by the top-level directory name. Errors on
    /// individual files are logged but do not abort the walk - the archive
    /// will still be produced without the failing entry
    ///
    /// Parallelization is intentionally avoided here: archive formats like tar
    /// require sequential append, and concurrent writes would corrupt the output
    ///
    /// # Arguments
    ///
    /// * `writer` - The archive writer to add directory entries to
    /// * `dir_path` - The directory on disk to walk and archive
    fn add_directory<W: Write + std::io::Seek>(
        &self,
        writer: &mut OmniWriter<W>,
        dir_path: &Path,
    ) -> std::io::Result<()> {
        // --------------------------------------------------
        // resolve the top-level directory name used as the
        // archive path prefix for all entries inside it
        // --------------------------------------------------
        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_DIR_NAME);
        // --------------------------------------------------
        // walk the directory sequentially and add each entry
        // --------------------------------------------------
        for entry in walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();
            // --------------------------------------------------
            // strip the dir prefix to get the relative path;
            // skip the root entry itself (empty relative path)
            // --------------------------------------------------
            let relative_path = match entry_path.strip_prefix(dir_path) {
                Ok(path) if path.as_os_str().is_empty() => continue,
                Ok(path) => path,
                Err(_) => continue,
            };
            let archive_path = format!("{}/{}", dir_name, relative_path.to_string_lossy());
            // --------------------------------------------------
            // add a directory entry or a file entry depending
            // on the entry's file type
            // --------------------------------------------------
            if entry.file_type().is_dir() {
                if let Err(e) = writer.add_directory(&archive_path) {
                    tracing::error!("Failed to add directory to archive: {e}");
                }
            } else {
                // --------------------------------------------------
                // read file contents then write into the archive
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
        Ok(())
    }

    /// Adds a single top-level file to the archive
    ///
    /// Reads the file at `file_path` into memory and passes it to the writer
    /// under its bare filename (no directory prefix)
    ///
    /// # Arguments
    ///
    /// * `writer` - The archive writer to add the file entry to
    /// * `file_path` - Path on disk to the file to archive
    fn add_file<W: Write + std::io::Seek>(
        &self,
        writer: &mut OmniWriter<W>,
        file_path: &Path,
    ) -> std::io::Result<()> {
        // --------------------------------------------------
        // resolve the bare filename used as the archive entry
        // name (falls back to DEFAULT_FILE_NAME if missing)
        // --------------------------------------------------
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_FILE_NAME);
        // --------------------------------------------------
        // read file contents into memory and add to archive
        // --------------------------------------------------
        let file_contents = std::fs::read(file_path)?;
        writer.add_file(filename, &file_contents)
    }
}
