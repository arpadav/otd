//! HTTP server implementation for the OTD application.
//!
//! This module implements a lightweight HTTP server using the `smol` async runtime.
//! It provides dual-port functionality with separate servers for admin interface
//! and download functionality.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use crate::{
    config::{Config, ParsedConfig},
    handlers::Handler,
    handlers::download::ARCHIVE_CACHE_DIR,
    types::AppState,
};
use smol::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener};
use std::{path::PathBuf, sync::Arc};

/// Initial buffer capacity for reading HTTP requests.
const READ_BUF_SIZE: usize = 4096;
/// Maximum number of HTTP headers to parse.
const MAX_PARSE_HEADERS: usize = 64;

/// Reads a complete HTTP request from `stream`.
///
/// Reads until the full request (headers + body) has been received, using
/// the `Content-Length` header to determine when the body is complete.
/// Returns an error if the total request size exceeds `max_bytes`.
///
/// # Returns
///
/// - `Ok(Some(data))` - complete request bytes
/// - `Ok(None)` - connection closed before any data was sent
/// - `Err(_)` - I/O error or request exceeded `max_bytes`
async fn read_request<S>(
    stream: &mut S,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>
where
    S: AsyncReadExt + Unpin,
{
    let mut buf: Vec<u8> = Vec::with_capacity(READ_BUF_SIZE);
    let mut tmp = [0u8; READ_BUF_SIZE];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Ok(if buf.is_empty() { None } else { Some(buf) });
        }
        if buf.len() + n > max_bytes {
            return Err(format!("Request exceeds maximum size of {max_bytes} bytes").into());
        }
        buf.extend_from_slice(&tmp[..n]);
        // --------------------------------------------------
        // check if we have complete headers yet (look for \r\n\r\n)
        // --------------------------------------------------
        if let Some(header_end) = find_header_end(&buf) {
            // --------------------------------------------------
            // parse Content-Length from the headers we have so far
            // --------------------------------------------------
            let content_length = parse_content_length(&buf[..header_end]);
            let body_received = buf.len().saturating_sub(header_end + 4);
            if body_received >= content_length {
                return Ok(Some(buf));
            }
        }
    }
}

/// Returns the byte offset of the end of HTTP headers (`\r\n\r\n`), or `None`.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Extracts the `Content-Length` value from raw HTTP headers.
/// Returns 0 if the header is absent or unparseable (correct for GET/HEAD).
fn parse_content_length(header_bytes: &[u8]) -> usize {
    let mut headers = [httparse::EMPTY_HEADER; MAX_PARSE_HEADERS];
    let mut req = httparse::Request::new(&mut headers);
    if req.parse(header_bytes).is_err() {
        return 0;
    }
    req.headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

/// Main server structure that manages both admin and download HTTP servers.
///
/// The server runs two separate HTTP servers on different ports:
/// - Admin server: Handles file browsing, link generation, and management
/// - Download server: Handles file downloads using generated tokens
///
/// Both servers share the same application state and configuration but serve
/// different purposes for security and organizational reasons.
///
/// # Examples
///
/// ```rust,no_run
/// use otd::{Server, Config};
/// use smol_macros::main;
/// use macro_rules_attribute::apply;
///
/// #[apply(main!)]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let config = Config::load()?;
///     let server = Server::new(config)?;
///     server.run().await
/// }
/// ```
pub struct Server {
    /// Pre-computed configuration (shared with handler via Arc)
    config: Arc<ParsedConfig>,
    /// Handler instance that processes incoming requests for both servers
    handler: Handler,
}

impl Server {
    /// Creates a new server instance with the provided configuration.
    ///
    /// Initializes the shared application state and creates a handler instance
    /// that will be used by both the admin and download servers.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration including ports, paths, and settings
    ///
    /// # Returns
    ///
    /// * `Server` - New server instance ready to run
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::{Server, Config};
    ///
    /// let server = Server::new(Config::default()).unwrap();
    /// ```
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let parsed = config.parse()?;
        // Canonicalize base_path at startup so all subsequent path checks
        // work against a resolved, symlink-free root. Fall back to the raw
        // path if canonicalization fails (e.g., directory doesn't exist yet).
        let raw_path = PathBuf::from(&parsed.raw.base_path);
        let base_path = std::fs::canonicalize(&raw_path).unwrap_or(raw_path);
        std::fs::create_dir_all(ARCHIVE_CACHE_DIR).ok();
        let state = Arc::new(AppState::new(base_path));
        let handler = Handler::new(state, parsed);
        let config = Arc::clone(&handler.config);

        Ok(Self { config, handler })
    }

    /// Starts both HTTP servers and runs them concurrently.
    ///
    /// This method starts the admin server and download server on their
    /// respective ports and runs them concurrently using `smol::future::try_zip`.
    /// The method will run indefinitely until an error occurs or the process
    /// is terminated.
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Success or error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either server fails to bind to its configured port
    /// - Socket addresses cannot be parsed from configuration
    /// - Network I/O errors occur during server operation
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::{Server, Config};
    /// use smol_macros::main;
    /// use macro_rules_attribute::apply;
    ///
    /// #[apply(main!)]
    /// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ///     let config = Config::load()?;
    ///     let server = Server::new(config)?;
    ///     server.run().await
    /// }
    /// ```
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let admin_addr = self.config.admin_addr;
        let download_addr = self.config.download_addr;
        tracing::info!("Admin server listening on {admin_addr}");
        tracing::info!("Download server listening on {download_addr}");
        let base_path = &self.config.raw.base_path;
        tracing::info!("Base path: {base_path}");
        if self.config.raw.admin_token.is_none() {
            tracing::warn!(
                "Admin interface has NO authentication. \
                 Set `admin_token` in otd-config.toml and bind to a trusted interface."
            );
        }
        // Start both servers concurrently
        let admin_handler = self.handler.clone();
        let download_handler = self.handler.clone();
        let admin_server = self.run_admin_server(admin_addr, admin_handler);
        let download_server = self.run_download_server(download_addr, download_handler);
        // Run both servers concurrently
        smol::future::try_zip(admin_server, download_server).await?;
        Ok(())
    }

    /// Runs the admin HTTP server on the specified address.
    ///
    /// The admin server handles requests for the web interface, file browsing,
    /// download link generation, and configuration. Each incoming connection
    /// is handled in a separate async task.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address to bind the server to
    /// * `handler` - Handler instance for processing requests
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Success or error
    async fn run_admin_server(
        &self,
        addr: std::net::SocketAddr,
        handler: Handler,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (mut stream, peer_addr) = listener.accept().await?;
            let handler = handler.clone();

            smol::spawn(async move {
                let max_bytes = handler.config.raw.max_request_size;
                match read_request(&mut stream, max_bytes).await {
                    Ok(Some(bytes)) => {
                        let request_str = String::from_utf8_lossy(&bytes);
                        match handler.handle_admin_request(&request_str, peer_addr).await {
                            Ok(response_bytes) => {
                                if let Err(e) = stream.write_all(&response_bytes).await {
                                    tracing::error!("Failed to write admin response: {e}");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error handling admin request: {e}");
                                let error_response =
                                    crate::http::HttpResponse::internal_server_error().to_bytes();
                                let _ = stream.write_all(&error_response).await;
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("Empty admin request received");
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Admin request read error (possible oversized request): {e}"
                        );
                        let response = crate::http::HttpResponse::payload_too_large().to_bytes();
                        let _ = stream.write_all(&response).await;
                    }
                }
            })
            .detach();
        }
    }

    /// Runs the download HTTP server on the specified address.
    ///
    /// The download server handles only file download requests using tokens
    /// generated by the admin interface. This separation provides better
    /// security by isolating download functionality from administrative functions.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address to bind the server to
    /// * `handler` - Handler instance for processing requests
    ///
    /// # Returns
    ///
    /// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - Success or error
    async fn run_download_server(
        &self,
        addr: std::net::SocketAddr,
        handler: Handler,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (mut stream, peer_addr) = listener.accept().await?;
            let handler = handler.clone();

            smol::spawn(async move {
                let max_bytes = handler.config.raw.max_request_size;
                match read_request(&mut stream, max_bytes).await {
                    Ok(Some(bytes)) => {
                        let request_str = String::from_utf8_lossy(&bytes);
                        match handler.handle_download_request(&request_str, peer_addr).await {
                            Ok(response_bytes) => {
                                if let Err(e) = stream.write_all(&response_bytes).await {
                                    tracing::error!("Failed to write download response: {e}");
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error handling download request: {e}");
                                let error_response =
                                    crate::http::HttpResponse::internal_server_error().to_bytes();
                                let _ = stream.write_all(&error_response).await;
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("Empty download request received");
                    }
                    Err(e) => {
                        tracing::warn!("Download request read error: {e}");
                        let response = crate::http::HttpResponse::payload_too_large().to_bytes();
                        let _ = stream.write_all(&response).await;
                    }
                }
            })
            .detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = Config::default();
        let server = Server::new(config).unwrap();

        assert_eq!(server.config.raw.admin_port, 15204);
        assert_eq!(server.config.raw.download_port, 15205);
    }

    #[test]
    fn test_config_addresses() {
        let config = Config::default();

        let admin_addr = config.admin_addr().unwrap();
        assert_eq!(admin_addr.port(), 15204);

        let download_addr = config.download_addr().unwrap();
        assert_eq!(download_addr.port(), 15205);
    }

    #[test]
    fn test_find_header_end_found() {
        // "GET / HTTP/1.1\r\nHost: x\r\n" is 26 bytes, \r\n\r\n starts at index 23
        let data = b"GET / HTTP/1.1\r\nHost: x\r\n\r\nbody";
        assert_eq!(find_header_end(data), Some(23));
    }

    #[test]
    fn test_find_header_end_not_found() {
        let data = b"GET / HTTP/1.1\r\nHost: x\r\n";
        assert_eq!(find_header_end(data), None);
    }

    #[test]
    fn test_parse_content_length_present() {
        let headers = b"POST /api/generate HTTP/1.1\r\nContent-Length: 42\r\nHost: x\r\n\r\n";
        assert_eq!(parse_content_length(headers), 42);
    }

    #[test]
    fn test_parse_content_length_absent() {
        let headers = b"GET / HTTP/1.1\r\nHost: x\r\n\r\n";
        assert_eq!(parse_content_length(headers), 0);
    }

    #[test]
    fn test_parse_content_length_case_insensitive() {
        let headers = b"POST / HTTP/1.1\r\ncontent-length: 99\r\n\r\n";
        assert_eq!(parse_content_length(headers), 99);
    }

    /// read_request should return the full request when Content-Length matches.
    #[test]
    fn test_read_request_complete_body() {
        let body = b"{\"paths\":[\"file.txt\"]}";
        let request = format!(
            "POST /api/generate HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        let mut cursor = smol::io::Cursor::new(request.as_bytes().to_vec());
        let result = smol::block_on(read_request(&mut cursor, 65536)).unwrap();
        assert!(result.is_some());
        let data = result.unwrap();
        assert!(data.ends_with(body));
    }

    /// read_request should return error when request exceeds max_bytes.
    #[test]
    fn test_read_request_exceeds_max() {
        let large_body = vec![b'x'; 200];
        let request = format!(
            "POST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
            large_body.len(),
            String::from_utf8(large_body).unwrap()
        );
        let mut cursor = smol::io::Cursor::new(request.as_bytes().to_vec());
        // Set max to 100 bytes - smaller than the request
        let result = smol::block_on(read_request(&mut cursor, 100));
        assert!(result.is_err(), "Should error on oversized request");
    }

    /// read_request on empty stream returns None.
    #[test]
    fn test_read_request_empty_stream() {
        let mut cursor = smol::io::Cursor::new(Vec::<u8>::new());
        let result = smol::block_on(read_request(&mut cursor, 65536)).unwrap();
        assert!(result.is_none());
    }
}
