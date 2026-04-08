//! Generic archive / compressed file
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::core::archive::{
    ArchiveWriter, CompressionType, DEFAULT_DIR_NAME, DEFAULT_FILE_NAME, OmniWriter,
};

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
    pub(crate) fn write_to_file(&self, output: &Path) -> std::io::Result<std::fs::File> {
        let file = std::fs::File::create(output)?;
        let mut writer = OmniWriter::new(self.compression, file);
        self.add_paths(&mut writer)?;
        writer.finish()
    }

    /// Builds the archive in memory and returns the raw bytes
    pub(crate) fn write_to_memory(
        &self,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = OmniWriter::new(self.compression, cursor);
            self.add_paths(&mut writer)?;
            writer.finish()?;
        }
        Ok(buf)
    }

    /// Ensures the archive name has the correct extension for this format
    pub(crate) fn ensure_extension<'a>(&self, name: &'a str) -> std::borrow::Cow<'a, str> {
        let ext = self.compression.extension();
        if name.ends_with(ext) {
            std::borrow::Cow::Borrowed(name)
        } else {
            std::borrow::Cow::Owned(format!("{name}{ext}"))
        }
    }

    /// Walks all source paths and feeds files/directories into the writer
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

    /// Recursively adds a directory and its contents to the archive
    ///
    /// Note that parallelization here could mess up the archive, since
    /// a sequential dir walker for folder/file associations is used
    ///
    /// I could be wrong? aav
    fn add_directory<W: Write + std::io::Seek>(
        &self,
        writer: &mut OmniWriter<W>,
        dir_path: &Path,
    ) -> std::io::Result<()> {
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
        writer: &mut OmniWriter<W>,
        file_path: &Path,
    ) -> std::io::Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_FILE_NAME);
        let file_contents = std::fs::read(file_path)?;
        writer.add_file(filename, &file_contents)
    }
}
