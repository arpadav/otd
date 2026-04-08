//! Format-specific archive writers
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::core::archive::{ARCHIVE_UNIX_PERMISSIONS, ArchiveWriter, CompressionType};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::io::Write;
use zip::{result::ZipError, write::FileOptions};

/// Tar archive operations shared by both compressed and uncompressed variants
pub(crate) struct TarWriter<W: Write>(tar::Builder<W>);
/// [`TarWriter`] implementation of [`ArchiveWriter`]
impl<W: Write> TarWriter<W> {
    /// Adds a file to the archive
    fn add_file(&mut self, archive_path: &str, contents: &[u8]) -> std::io::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(ARCHIVE_UNIX_PERMISSIONS);
        header.set_cksum();
        self.0.append_data(&mut header, archive_path, contents)?;
        Ok(())
    }

    /// Adds a directory to the archive
    fn add_directory(&mut self, archive_path: &str) -> std::io::Result<()> {
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

    #[inline(always)]
    /// Finishes the archive and returns the underlying writer
    fn finish(self) -> std::io::Result<W> {
        self.0.into_inner()
    }
}

/// Wraps a format-specific archive writer so callers don't need to know
/// which compression backend is in use
pub(crate) enum OmniWriter<W: Write + std::io::Seek> {
    /// zip
    Zip(Box<zip::ZipWriter<W>>),
    /// tar
    Tar(Box<TarWriter<W>>),
    /// tar.gz
    TarGz(Box<TarWriter<flate2::write::GzEncoder<W>>>),
}
/// [`OmniWriter`] implementation
impl<W> OmniWriter<W>
where
    W: Write + std::io::Seek,
{
    /// Creates a new archive writer for the given compression type
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
    /// See [`ArchiveWriter::add_file`]
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

    /// See [`ArchiveWriter::add_directory`]
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

    /// See [`ArchiveWriter::finish`]
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
