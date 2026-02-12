//! Configuration management for the OTD server.
//!
//! This module handles loading and managing server configuration from TOML files
//! and environment variables. It provides sensible defaults and automatic config
//! file generation.

use serde::{Deserialize, Serialize};
use std::{fs, net::SocketAddr, path::PathBuf};

/// Main configuration structure for the OTD server.
///
/// Contains all configurable parameters including network settings, paths,
/// security options, and performance tuning. Configuration is loaded from
/// a TOML file with environment variable overrides.
///
/// # Examples
///
/// ```rust
/// use otd::Config;
///
/// // Load configuration from file or create default
/// let config = Config::load().unwrap();
/// println!("Admin port: {}", config.admin_port);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Port for the admin interface (file browsing, link generation)
    pub admin_port: u16,
    /// Port for the download server (file downloads only)
    pub download_port: u16,
    /// Host/IP address for the admin interface.
    /// Defaults to `127.0.0.1` — intentionally localhost-only.
    /// Set to `0.0.0.0` only if you are behind a trusted reverse proxy
    /// that enforces its own authentication.
    pub admin_host: String,
    /// Host/IP address for the download server
    pub download_host: String,
    /// Base directory path for file serving
    pub base_path: String,
    /// Buffer size for HTTP request reading
    pub buffer_size: usize,
    /// Maximum allowed request size in bytes
    pub max_request_size: usize,
    /// Whether HTTPS is enabled
    pub enable_https: bool,
    /// Path to TLS certificate file (required if HTTPS enabled)
    pub cert_path: Option<String>,
    /// Path to TLS private key file (required if HTTPS enabled)
    pub key_path: Option<String>,
    /// Optional shared secret token for admin interface authentication.
    /// When set, every admin request must include the header:
    ///   `Authorization: Bearer <token>`
    /// Leave `None` to disable authentication (only safe on localhost).
    pub admin_token: Option<String>,
    /// Optional password for admin interface login from non-loopback addresses.
    /// When set, external (non-127.0.0.1/::1) requests must authenticate via
    /// a login form. When `None`, external requests receive a 403 error.
    pub admin_password: Option<String>,
}

impl Default for Config {
    /// Creates a default configuration with sensible values.
    ///
    /// # Default Values
    ///
    /// - Admin port: 15204
    /// - Download port: 15205
    /// - Admin host: 127.0.0.1 (localhost only)
    /// - Download host: otd-hostname
    /// - Base path: current directory
    /// - Buffer size: 8KB
    /// - Max request size: 10MB
    /// - HTTPS: disabled
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    ///
    /// let config = Config::default();
    /// assert_eq!(config.admin_port, 15204);
    /// assert_eq!(config.download_port, 15205);
    /// ```
    fn default() -> Self {
        Self {
            admin_port: 15204,
            download_port: 15205,
            // Admin defaults to loopback — the admin interface has no auth
            // by default and must not be exposed to the network unprotected.
            admin_host: "127.0.0.1".to_string(),
            download_host: "otd-hostname".to_string(),
            base_path: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/tmp".to_string()),
            buffer_size: 8192,
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_https: false,
            cert_path: None,
            key_path: None,
            admin_token: None,
            admin_password: None,
        }
    }
}

impl Config {
    /// Loads configuration from file or creates a default configuration.
    ///
    /// This method attempts to load configuration from `otd-config.toml` in the
    /// current directory. If the file doesn't exist, it creates a default
    /// configuration file and returns the default values.
    ///
    /// Environment variables can override configuration values:
    /// - `OTD_BASE_PATH`: Overrides the base_path setting
    ///
    /// # Returns
    ///
    /// * `Result<Config, Box<dyn std::error::Error + Send + Sync>>` - Loaded configuration or error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::Config;
    ///
    /// let config = Config::load().unwrap();
    /// println!("Base path: {}", config.base_path);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration file exists but cannot be read
    /// - The configuration file contains invalid TOML syntax
    /// - The default configuration cannot be written to disk
    pub fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = "otd-config.toml";
        
        if !PathBuf::from(config_path).exists() {
            let default_config = Self::default();
            let toml_str = toml::to_string_pretty(&default_config)?;
            fs::write(config_path, toml_str)?;
            tracing::info!("Created default config file: {}", config_path);
            return Ok(default_config);
        }

        let config_str = fs::read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&config_str)?;
        
        // Override with environment variables if present
        if let Ok(base_path) = std::env::var("OTD_BASE_PATH") {
            config.base_path = base_path;
        }
        
        Ok(config)
    }

    /// Returns the socket address for the admin interface.
    ///
    /// Combines the admin host and port into a `SocketAddr` for binding.
    ///
    /// # Returns
    ///
    /// * `Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>>` - Socket address or parsing error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    ///
    /// let config = Config::default();
    /// let addr = config.admin_addr().unwrap();
    /// assert_eq!(addr.port(), 15204);
    /// ```
    pub fn admin_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        let addr_str = format!("{}:{}", self.admin_host, self.admin_port);
        Ok(addr_str.parse()?)
    }

    /// Returns the socket address for the download server.
    ///
    /// Combines the download host and port into a `SocketAddr` for binding.
    ///
    /// # Returns
    ///
    /// * `Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>>` - Socket address or parsing error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    ///
    /// let config = Config::default();
    /// let addr = config.download_addr().unwrap();
    /// assert_eq!(addr.port(), 15205);
    /// ```
    pub fn download_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        let addr_str = format!("{}:{}", self.download_host, self.download_port);
        Ok(addr_str.parse()?)
    }

    /// Returns the base URL for download links.
    ///
    /// Constructs the complete base URL including protocol, host, and port
    /// for generating download links. Uses HTTPS if enabled, HTTP otherwise.
    ///
    /// # Returns
    ///
    /// * `String` - Complete base URL (e.g., "http://otd-hostname:15205")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    ///
    /// let config = Config::default();
    /// let base_url = config.download_base_url();
    /// assert_eq!(base_url, "http://otd-hostname:15205");
    /// ```
    pub fn download_base_url(&self) -> String {
        let protocol = if self.enable_https { "https" } else { "http" };
        format!("{}://{}:{}", protocol, self.download_host, self.download_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.admin_port, 15204);
        assert_eq!(config.download_port, 15205);
        // Admin must default to loopback for safety.
        assert_eq!(config.admin_host, "127.0.0.1");
        assert_eq!(config.download_host, "otd-hostname");
        assert!(!config.enable_https);
        // No token by default — users should set one if exposing over network.
        assert!(config.admin_token.is_none());
    }

    #[test]
    fn test_socket_addresses() {
        let config = Config::default();
        
        let admin_addr = config.admin_addr().unwrap();
        assert_eq!(admin_addr.port(), 15204);
        
        let download_addr = config.download_addr().unwrap();
        assert_eq!(download_addr.port(), 15205);
    }

    #[test]
    fn test_download_base_url() {
        let mut config = Config::default();
        assert_eq!(config.download_base_url(), "http://otd-hostname:15205");
        
        config.enable_https = true;
        assert_eq!(config.download_base_url(), "https://otd-hostname:15205");
        
        config.download_host = "example.com".to_string();
        config.download_port = 443;
        assert_eq!(config.download_base_url(), "https://example.com:443");
    }
}