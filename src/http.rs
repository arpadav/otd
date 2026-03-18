//! HTTP response building utilities.
//!
//! This module provides a clean, type-safe way to build HTTP responses without
//! relying on heavy web frameworks. It includes proper status codes, headers,
//! and content handling.
//!
//! # Examples
//!
//! ```rust
//! use otd::http::{HttpResponse, content_type};
//!
//! // Simple text response
//! let response = HttpResponse::ok()
//!     .content_type(content_type::PLAIN_TEXT)
//!     .body_text("Hello, world!");
//!
//! // JSON response
//! let data = serde_json::json!({"status": "success"});
//! let response = HttpResponse::ok()
//!     .body_json(&data)
//!     .unwrap();
//!
//! // File download with proper headers
//! let file_data = b"Example file content".to_vec();
//! let response = HttpResponse::ok()
//!     .content_type(content_type::OCTET_STREAM)
//!     .content_disposition("attachment; filename=\"example.txt\"")
//!     .body_bytes(file_data);
//! ```
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use std::collections::HashMap;

/// HTTP response builder that provides a fluent interface for constructing responses.
///
/// This struct allows you to build HTTP responses with proper status codes, headers,
/// and body content. It's designed to be lightweight and avoid common pitfalls
/// like missing Content-Length headers or improper content types.
///
/// # Examples
///
/// ```rust
/// use otd::http::HttpResponse;
///
/// let response = HttpResponse::ok()
///     .content_type("text/plain")
///     .body_text("Hello, world!");
///
/// let bytes = response.to_bytes();
/// assert!(String::from_utf8_lossy(&bytes).contains("Hello, world!"));
/// ```
pub struct HttpResponse {
    /// HTTP status code (e.g., 200, 404, 500)
    status_code: u16,
    /// HTTP status text (e.g., "OK", "Not Found", "Internal Server Error")
    status_text: &'static str,
    /// HTTP headers as key-value pairs
    headers: HashMap<String, String>,
    /// Response body as raw bytes
    body: Vec<u8>,
}
/// [`HttpResponse`] implementation
impl HttpResponse {
    /// Creates a new HTTP response with the specified status code and text.
    ///
    /// # Arguments
    ///
    /// * `status_code` - HTTP status code (e.g., 200, 404, 500)
    /// * `status_text` - HTTP status text (e.g., "OK", "Not Found", "Internal Server Error")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::new(200, "OK");
    /// ```
    pub fn new(status_code: u16, status_text: &'static str) -> Self {
        Self {
            status_code,
            status_text,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Adds a header to the response.
    ///
    /// # Arguments
    ///
    /// * `key` - Header name (e.g., "Content-Type", "Cache-Control")
    /// * `value` - Header value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok()
    ///     .header("Cache-Control", "no-cache")
    ///     .header("X-Custom-Header", "custom-value");
    /// ```
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets the Content-Type header.
    ///
    /// # Arguments
    ///
    /// * `content_type` - MIME type (e.g., "text/html", "application/json")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::{HttpResponse, content_type};
    ///
    /// let response = HttpResponse::ok()
    ///     .content_type(content_type::JSON);
    /// ```
    pub fn content_type(self, content_type: &str) -> Self {
        self.header("Content-Type", content_type)
    }

    /// Sets the Content-Disposition header, typically used for file downloads.
    ///
    /// # Arguments
    ///
    /// * `disposition` - Content disposition value (e.g., "attachment; filename=\"file.txt\"")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok()
    ///     .content_disposition("attachment; filename=\"document.pdf\"");
    /// ```
    pub fn content_disposition(self, disposition: &str) -> Self {
        self.header("Content-Disposition", disposition)
    }

    /// Sets the response body to the provided text and automatically sets Content-Length.
    ///
    /// # Arguments
    ///
    /// * `text` - Text content for the response body
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok()
    ///     .body_text("Hello, world!");
    /// ```
    pub fn body_text(mut self, text: &str) -> Self {
        self.body = text.as_bytes().to_vec();
        let content_length = self.body.len().to_string();
        self.header("Content-Length", &content_length)
    }

    /// Sets the response body to JSON-serialized data and sets appropriate headers.
    ///
    /// # Arguments
    ///
    /// * `data` - Any serializable data structure
    ///
    /// # Returns
    ///
    /// * `Result<Self, serde_json::Error>` - The response builder or serialization error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    /// use serde_json::json;
    ///
    /// let data = json!({"message": "success", "code": 200});
    /// let response = HttpResponse::ok()
    ///     .body_json(&data)
    ///     .unwrap();
    /// ```
    pub fn body_json<T: serde::Serialize>(mut self, data: &T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_string(data)?;
        self.body = json.as_bytes().to_vec();
        let content_length = self.body.len().to_string();
        Ok(self
            .content_type("application/json")
            .header("Content-Length", &content_length))
    }

    /// Sets the response body to raw bytes and automatically sets Content-Length.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Raw byte data for the response body
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let file_data = b"Binary file content".to_vec();
    /// let response = HttpResponse::ok()
    ///     .content_type("application/octet-stream")
    ///     .body_bytes(file_data);
    /// ```
    pub fn body_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.body = bytes;
        let content_length = self.body.len().to_string();
        self.header("Content-Length", &content_length)
    }

    /// Initializes the response with status line and headers, but without body.
    ///
    /// Used internally to prepare the response string before appending the body content.
    ///
    /// # Returns
    ///
    /// [`String`] containing the HTTP status line and headers, ready to be sent before the body.
    fn init_response(&self) -> String {
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);
        for (key, value) in &self.headers {
            response.push_str(&format!("{key}: {value}\r\n"));
        }
        response.push_str("\r\n");
        response
    }

    /// Converts the response to a string representation (useful for text responses).
    ///
    /// # Returns
    ///
    /// Complete HTTP response as a string, including headers and body.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok().body_text("Hello");
    /// let response_str = response.to_string_response();
    /// assert!(response_str.contains("HTTP/1.1 200 OK"));
    /// assert!(response_str.contains("Hello"));
    /// ```
    pub fn to_string_response(self) -> String {
        let mut response = self.init_response();
        response.push_str(&String::from_utf8_lossy(&self.body));
        response
    }

    /// Converts the response to raw bytes (recommended for binary responses).
    ///
    /// # Returns
    ///
    /// Complete HTTP response as bytes, including headers and body.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok().body_text("Hello");
    /// let bytes = response.to_bytes();
    /// assert!(bytes.len() > 0);
    /// ```
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = self.init_response().into_bytes();
        bytes.extend(self.body);
        bytes
    }
}

/// HTTP status code constants.
///
/// Provides commonly used HTTP status codes as tuples of (code, text).
pub mod status {
    /// 200 OK - Request succeeded
    pub const OK: (u16, &str) = (200, "OK");
    /// 400 Bad Request - Invalid request syntax
    pub const BAD_REQUEST: (u16, &str) = (400, "Bad Request");
    /// 403 Forbidden - Server understood but refuses to authorize
    pub const FORBIDDEN: (u16, &str) = (403, "Forbidden");
    /// 404 Not Found - Requested resource not found
    pub const NOT_FOUND: (u16, &str) = (404, "Not Found");
    /// 410 Gone - Resource no longer available
    pub const GONE: (u16, &str) = (410, "Gone");
    /// 500 Internal Server Error - Server encountered an error
    pub const INTERNAL_SERVER_ERROR: (u16, &str) = (500, "Internal Server Error");
}

/// Content type constants for common MIME types.
///
/// Provides commonly used content types to avoid typos and ensure consistency.
pub mod content_type {
    /// HTML content with UTF-8 encoding
    pub const HTML: &str = "text/html; charset=utf-8";
    /// JSON content
    pub const JSON: &str = "application/json";
    /// Binary/unknown content type
    pub const OCTET_STREAM: &str = "application/octet-stream";
    /// ZIP archive
    pub const ZIP: &str = "application/zip";
    /// Plain text
    pub const PLAIN_TEXT: &str = "text/plain";
}
/// [`HttpResponse`] implementation
impl HttpResponse {
    /// Creates a 200 OK response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::ok().body_text("Success!");
    /// ```
    pub fn ok() -> Self {
        Self::new(status::OK.0, status::OK.1)
    }

    /// Creates a 400 Bad Request response with default message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::bad_request();
    /// ```
    pub fn bad_request() -> Self {
        Self::new(status::BAD_REQUEST.0, status::BAD_REQUEST.1).body_text("Bad Request")
    }

    /// Creates a 403 Forbidden response with default message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::forbidden();
    /// ```
    pub fn forbidden() -> Self {
        Self::new(status::FORBIDDEN.0, status::FORBIDDEN.1).body_text("Forbidden")
    }

    /// Creates a 404 Not Found response with default message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::not_found();
    /// ```
    pub fn not_found() -> Self {
        Self::new(status::NOT_FOUND.0, status::NOT_FOUND.1).body_text("Not Found")
    }

    /// Creates a 410 Gone response with message about expired download links.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::gone();
    /// ```
    pub fn gone() -> Self {
        Self::new(status::GONE.0, status::GONE.1).body_text("Download link has already been used")
    }

    /// Creates a 302 redirect response.
    ///
    /// # Arguments
    ///
    /// * `location` - URL to redirect to
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::redirect("/login");
    /// ```
    pub fn redirect(location: &str) -> Self {
        Self::new(302, "Found")
            .header("Location", location)
            .body_text("")
    }

    /// Creates a 500 Internal Server Error response with default message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::http::HttpResponse;
    ///
    /// let response = HttpResponse::internal_server_error();
    /// ```
    pub fn internal_server_error() -> Self {
        Self::new(
            status::INTERNAL_SERVER_ERROR.0,
            status::INTERNAL_SERVER_ERROR.1,
        )
        .body_text("Internal Server Error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_response() {
        let response = HttpResponse::ok().body_text("Hello");
        let bytes = response.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Content-Length: 5"));
        assert!(response_str.contains("Hello"));
    }

    #[test]
    fn test_json_response() {
        let data = serde_json::json!({"test": "value"});
        let response = HttpResponse::ok().body_json(&data).unwrap();
        let bytes = response.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("application/json"));
        assert!(response_str.contains("test"));
        assert!(response_str.contains("value"));
    }

    #[test]
    fn test_file_download_headers() {
        let response = HttpResponse::ok()
            .content_type(content_type::OCTET_STREAM)
            .content_disposition("attachment; filename=\"test.txt\"")
            .body_bytes(b"file content".to_vec());

        let bytes = response.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.contains("application/octet-stream"));
        assert!(response_str.contains("attachment; filename=\"test.txt\""));
        assert!(response_str.contains("Content-Length: 12"));
    }

    #[test]
    fn test_error_responses() {
        let not_found = HttpResponse::not_found();
        let bytes = not_found.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);
        assert!(response_str.contains("404 Not Found"));

        let gone = HttpResponse::gone();
        let bytes = gone.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);
        assert!(response_str.contains("410 Gone"));
        assert!(response_str.contains("already been used"));
    }
}
