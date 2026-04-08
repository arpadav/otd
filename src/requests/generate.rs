//! Request and response types for the generate-link API endpoint
//!
//! Used by the [`generate_link`][crate::core::links::generate_link] server function
//! and the browse page client code
//!
//! Author: aav
use crate::core::archive::CompressionType;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
/// Request payload for generating new download links
///
/// # Examples
///
/// ```
/// use otd::requests::GenerateRequest;
///
/// let json = r#"{
///   "paths": ["/home/user/file.txt"],
///   "name": "my-download",
///   "max_downloads": 3,
///   "expires_in_seconds": 3600
/// }"#;
/// let req: GenerateRequest = serde_json::from_str(json).unwrap();
/// assert_eq!(req.paths, vec!["/home/user/file.txt"]);
/// assert_eq!(req.name.as_deref(), Some("my-download"));
/// ```
pub struct GenerateRequest {
    /// List of file paths to include in the download
    pub paths: Vec<String>,

    /// Optional custom name for the download archive
    pub name: Option<String>,

    /// Optional maximum number of downloads allowed
    pub max_downloads: Option<u32>,

    /// Optional number of seconds until the download expires
    pub expires_in_seconds: Option<u64>,

    #[serde(default)]
    /// Archive format (defaults to [`CompressionType::Zip`] when absent)
    pub format: CompressionType,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
/// Response payload when a download link is successfully generated
///
/// # Examples
///
/// ```
/// use otd::requests::GenerateResponse;
///
/// let json = r#"{"token":"abc123","download_url":"https://example.com/dl/file.zip?k=abc123"}"#;
/// let resp: GenerateResponse = serde_json::from_str(json).unwrap();
/// assert_eq!(resp.token, "abc123");
/// ```
pub struct GenerateResponse {
    /// Unique identifier for this download
    pub token: String,
    /// Complete URL for downloading the file(s)
    pub download_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_request_serde_roundtrip() {
        let req = GenerateRequest {
            paths: vec!["/tmp/file.txt".into()],
            name: Some("archive".into()),
            max_downloads: Some(5),
            expires_in_seconds: Some(7200),
            format: CompressionType::TarGz,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: GenerateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.paths, req.paths);
        assert_eq!(decoded.name, req.name);
        assert_eq!(decoded.max_downloads, req.max_downloads);
        assert_eq!(decoded.expires_in_seconds, req.expires_in_seconds);
        assert_eq!(decoded.format, CompressionType::TarGz);
    }

    #[test]
    fn generate_request_format_defaults_to_zip() {
        let json = r#"{"paths":["/tmp/a.txt"]}"#;
        let req: GenerateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.format, CompressionType::Zip);
    }

    #[test]
    fn generate_response_serde_roundtrip() {
        let resp = GenerateResponse {
            token: "tok-xyz".into(),
            download_url: "http://localhost/dl/f.zip?k=tok-xyz".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GenerateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.token, resp.token);
        assert_eq!(decoded.download_url, resp.download_url);
    }
}
