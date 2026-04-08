//! Archive / compression formats
//!
//! Must update writers once adding a new format
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

    /// Tar archive
    Tar,
}
#[cfg(feature = "server")]
/// [`CompressionType`] implementation
impl CompressionType {
    /// File extension for this compression type (including the leading dot)
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Zip => ".zip",
            Self::TarGz => ".tar.gz",
            Self::Tar => ".tar",
        }
    }

    /// HTTP `Content-Type` header value for this format
    pub const fn content_type(&self) -> &'static str {
        match self {
            Self::Zip => "application/zip",
            Self::TarGz => "application/gzip",
            Self::Tar => "application/x-tar",
        }
    }
}
