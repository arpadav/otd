//! Format-specific archive writer backends
//!
//! Provides [`TarWriter`] (shared by both compressed and uncompressed tar)
//! and [`OmniWriter`], the format-dispatching wrapper that implements
//! [`ArchiveWriter`] and is used by [`CompressedFile`][super::CompressedFile]
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::{ARCHIVE_UNIX_PERMISSIONS, ArchiveWriter, CompressionType};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::io::Write;
use zip::{result::ZipError, write::FileOptions};

/// Tar archive operations shared by both compressed and uncompressed tar variants
pub(crate) struct TarWriter<W: Write>(tar::Builder<W>);

/// [`TarWriter`] implementation
impl<W: Write> TarWriter<W> {
    /// Appends a file entry to the tar archive
    ///
    /// Constructs a GNU tar header with the file size and unix permissions,
    /// computes the checksum, and appends the header and contents
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path the entry will have inside the archive
    /// * `contents` - Raw bytes of the file to append
    fn add_file(&mut self, archive_path: &str, contents: &[u8]) -> std::io::Result<()> {
        // --------------------------------------------------
        // build a gnu tar header with size, permissions, and
        // checksum set
        // --------------------------------------------------
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(ARCHIVE_UNIX_PERMISSIONS);
        header.set_cksum();
        // --------------------------------------------------
        // append the header and file contents to the archive
        // --------------------------------------------------
        self.0.append_data(&mut header, archive_path, contents)?;
        Ok(())
    }

    /// Appends a directory entry to the tar archive
    ///
    /// Constructs a GNU tar header with zero size, directory entry type, and
    /// unix permissions. Ensures the path ends with a trailing slash as
    /// required by the tar format for directory entries
    ///
    /// # Arguments
    ///
    /// * `archive_path` - The path the directory entry will have inside the archive
    fn add_directory(&mut self, archive_path: &str) -> std::io::Result<()> {
        // --------------------------------------------------
        // build a gnu tar header typed as a directory entry
        // --------------------------------------------------
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(ARCHIVE_UNIX_PERMISSIONS);
        header.set_entry_type(tar::EntryType::Directory);
        header.set_cksum();
        // --------------------------------------------------
        // tar requires directory paths to end with '/' - append
        // it if missing
        // --------------------------------------------------
        let dir_path = if archive_path.ends_with('/') {
            std::borrow::Cow::Borrowed(archive_path)
        } else {
            std::borrow::Cow::Owned(format!("{archive_path}/"))
        };
        // --------------------------------------------------
        // append the directory header with an empty body
        // --------------------------------------------------
        self.0
            .append_data(&mut header, dir_path.as_ref(), &[][..])?;
        Ok(())
    }

    #[inline(always)]
    /// Finalizes the tar archive and returns the underlying writer
    fn finish(self) -> std::io::Result<W> {
        self.0.into_inner()
    }
}

/// Wraps a format-specific archive writer so callers don't need to know
/// which compression backend is in use
///
/// Dispatches [`ArchiveWriter`] method calls to the correct backend based on
/// the [`CompressionType`] provided at construction
pub(crate) enum OmniWriter<W: Write + std::io::Seek> {
    /// ZIP archive writer
    Zip(Box<zip::ZipWriter<W>>),
    /// Uncompressed tar archive writer
    Tar(Box<TarWriter<W>>),
    /// Gzip-compressed tar archive writer
    TarGz(Box<TarWriter<flate2::write::GzEncoder<W>>>),
}

/// [`OmniWriter`] implementation
impl<W> OmniWriter<W>
where
    W: Write + std::io::Seek,
{
    /// Creates a new archive writer for the given compression type and destination
    ///
    /// Constructs the appropriate backend writer and wraps it in the correct
    /// [`OmniWriter`] variant. For `TarGz`, a `GzEncoder` is layered on top
    /// of the destination before being passed to `TarWriter`
    ///
    /// # Arguments
    ///
    /// * `compression` - The archive format to produce
    /// * `dest` - The destination writer to write the archive bytes into
    pub(crate) fn new(compression: CompressionType, dest: W) -> Self {
        match compression {
            CompressionType::Zip => Self::Zip(Box::new(zip::ZipWriter::new(dest))),
            CompressionType::TarGz => {
                let encoder = flate2::write::GzEncoder::new(dest, flate2::Compression::default());
                Self::TarGz(Box::new(TarWriter(tar::Builder::new(encoder))))
            }
            CompressionType::Tar => Self::Tar(Box::new(TarWriter(tar::Builder::new(dest)))),
        }
    }
}

/// [`OmniWriter`] implementation of [`ArchiveWriter`]
impl<W> ArchiveWriter<W> for OmniWriter<W>
where
    W: Write + std::io::Seek,
{
    fn add_file(&mut self, archive_path: &str, contents: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Zip(zip) => {
                let options = FileOptions::<()>::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(ARCHIVE_UNIX_PERMISSIONS);
                zip.start_file(archive_path, options)?;
                zip.write_all(contents)
            }
            Self::TarGz(tar) => tar.add_file(archive_path, contents),
            Self::Tar(tar) => tar.add_file(archive_path, contents),
        }
    }

    fn add_directory(&mut self, archive_path: &str) -> std::io::Result<()> {
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

    fn finish(self) -> std::io::Result<W> {
        match self {
            Self::Zip(zip) => zip.finish().map_err(|e| match e {
                ZipError::Io(e) => e,
                other => std::io::Error::other(other),
            }),
            Self::TarGz(tar) => tar.finish()?.finish(),
            Self::Tar(tar) => tar.finish(),
        }
    }
}
