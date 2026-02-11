//! HTTP server implementation for the OTD application.
//!
//! This module implements a lightweight HTTP server using the `smol` async runtime.
//! It provides dual-port functionality with separate servers for admin interface
//! and download functionality.

use crate::{config::Config, handlers::Handler, types::AppState};
use smol::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpListener};
use std::{path::PathBuf, sync::Arc};

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
///     let server = Server::new(config);
///     server.run().await
/// }
/// ```
pub struct Server {
    config: Config,
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
    /// let config = Config::default();
    /// let server = Server::new(config);
    /// ```
    pub fn new(config: Config) -> Self {
        // Canonicalize base_path at startup so all subsequent path checks
        // work against a resolved, symlink-free root. Fall back to the raw
        // path if canonicalization fails (e.g., directory doesn't exist yet).
        let raw_path = PathBuf::from(&config.base_path);
        let base_path = std::fs::canonicalize(&raw_path).unwrap_or(raw_path);
        let state = Arc::new(AppState::new(base_path));
        let handler = Handler::new(state, config.clone());
        
        Self { config, handler }
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
    ///     let server = Server::new(config);
    ///     server.run().await
    /// }
    /// ```
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let admin_addr = self.config.admin_addr()?;
        let download_addr = self.config.download_addr()?;
        
        tracing::info!("Admin server listening on {}", admin_addr);
        tracing::info!("Download server listening on {}", download_addr);
        tracing::info!("Base path: {}", self.config.base_path);

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
            let (mut stream, _) = listener.accept().await?;
            let handler = handler.clone();
            
            smol::spawn(async move {
                let mut buffer = vec![0; handler.config.buffer_size];
                match stream.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let request_str = String::from_utf8_lossy(&buffer[..n]);
                        
                        match handler.handle_admin_request(&request_str).await {
                            Ok(response_bytes) => {
                                if let Err(e) = stream.write_all(&response_bytes).await {
                                    tracing::error!("Failed to write admin response: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error handling admin request: {}", e);
                                let error_response = crate::http::HttpResponse::internal_server_error().to_bytes();
                                let _ = stream.write_all(&error_response).await;
                            }
                        }
                    }
                    Ok(_) => {
                        tracing::debug!("Empty admin request received");
                    }
                    Err(e) => {
                        tracing::error!("Failed to read from admin stream: {}", e);
                    }
                }
            }).detach();
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
            let (mut stream, _) = listener.accept().await?;
            let handler = handler.clone();
            
            smol::spawn(async move {
                let mut buffer = vec![0; handler.config.buffer_size];
                match stream.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let request_str = String::from_utf8_lossy(&buffer[..n]);
                        
                        match handler.handle_download_request(&request_str).await {
                            Ok(response_bytes) => {
                                if let Err(e) = stream.write_all(&response_bytes).await {
                                    tracing::error!("Failed to write download response: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error handling download request: {}", e);
                                let error_response = crate::http::HttpResponse::internal_server_error().to_bytes();
                                let _ = stream.write_all(&error_response).await;
                            }
                        }
                    }
                    Ok(_) => {
                        tracing::debug!("Empty download request received");
                    }
                    Err(e) => {
                        tracing::error!("Failed to read from download stream: {}", e);
                    }
                }
            }).detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let config = Config::default();
        let server = Server::new(config.clone());
        
        assert_eq!(server.config.admin_port, config.admin_port);
        assert_eq!(server.config.download_port, config.download_port);
    }

    #[test]
    fn test_config_addresses() {
        let config = Config::default();
        
        let admin_addr = config.admin_addr().unwrap();
        assert_eq!(admin_addr.port(), 15204);
        
        let download_addr = config.download_addr().unwrap();
        assert_eq!(download_addr.port(), 15205);
    }
}