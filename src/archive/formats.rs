//! Archive and compression format definitions
//!
//! Defines [`CompressionType`], the enum representing all supported archive
//! formats. When adding a new format, the corresponding writer backend must
//! also be added in `writers.rs`
//!
//! Author: aav

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// Supported archive/compression formats
pub enum CompressionType {
    #[default]
    /// Standard ZIP archive (deflate)
    Zip,

    #[serde(alias = "targz", alias = "tgz")]
    /// Gzip-compressed tar archive
    TarGz,

    /// Uncompressed tar archive
    Tar,
}

/// [`CompressionType`] implementation
impl CompressionType {
    #[inline(always)]
    /// Returns the file extension for this compression type, including the leading dot
    ///
    /// Used when constructing archive filenames and cache paths
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Zip => ".zip",
            Self::TarGz => ".tar.gz",
            Self::Tar => ".tar",
        }
    }

    #[inline(always)]
    /// Returns the HTTP `Content-Type` header value for this format
    ///
    /// Used when serving archive downloads to set the correct MIME type
    pub const fn content_type(&self) -> &'static str {
        match self {
            Self::Zip => "application/zip",
            Self::TarGz => "application/gzip",
            Self::Tar => "application/x-tar",
        }
    }
}
