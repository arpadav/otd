//! Configuration management for the OTD server.
//!
//! This module handles loading and managing server configuration from TOML files
//! and environment variables. It provides sensible defaults and automatic config
//! file generation.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};

// --------------------------------------------------
// constants
// --------------------------------------------------
const OTD_CONFIG_FILE: &str = "otd-config.toml";
const OTD_CONFIG_ENVIRONMENT_VAR: &str = "OTD_CONFIG_FILE";
const OTD_BASE_ENVIRONMENT_VAR: &str = "OTD_BASE_PATH";

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
    /// Host/IP address for the admin interface.
    /// Defaults to `127.0.0.1` - intentionally localhost-only.
    /// Set to `0.0.0.0` only if you are behind a trusted reverse proxy
    /// that enforces its own authentication.
    pub admin_host: String,
    /// Port for the download server (file downloads only)
    pub download_port: u16,
    /// Host/IP address for the download server
    pub download_host: String,
    /// Base url for download links (e.g., "https://files.example.com/")
    /// If `None`, will just default to the download host and port (e.g., "http://{download_host}:{download_port}").
    pub download_base_url: Option<String>,
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
/// [`Config`] implementation of [`Default`]
impl Default for Config {
    fn default() -> Self {
        Self {
            admin_port: 15204,
            admin_host: "127.0.0.1".into(),
            download_port: 15205,
            download_host: "0.0.0.0".into(),
            download_base_url: None,
            base_path: std::env::current_dir()
                .map(|p| p.to_string_lossy().into())
                .unwrap_or_else(|_| "/tmp".into()),
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
/// [`Config`] implementation
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
        // --------------------------------------------------
        // create default file if not exists
        // --------------------------------------------------
        let config_path = match (
            PathBuf::from(OTD_CONFIG_FILE).exists(),
            std::env::var(OTD_CONFIG_ENVIRONMENT_VAR),
        ) {
            (_, Ok(config_path)) => {
                tracing::info!(
                    "Using config file from environment variable {}: {}",
                    OTD_CONFIG_ENVIRONMENT_VAR,
                    config_path
                );
                config_path
            }
            (false, Err(_)) => {
                let default_config = Self::default();
                let toml_str = toml::to_string_pretty(&default_config)?;
                std::fs::write(OTD_CONFIG_FILE, toml_str)?;
                tracing::info!("Created default config file: {}", OTD_CONFIG_FILE);
                OTD_CONFIG_FILE.to_string()
            }
            (true, Err(_)) => OTD_CONFIG_FILE.to_string(),
        };
        let config_str = std::fs::read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&config_str)?;
        // --------------------------------------------------
        // override with environment variables if present
        // --------------------------------------------------
        if let Ok(base_path) = std::env::var(OTD_BASE_ENVIRONMENT_VAR) {
            config.base_path = base_path;
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
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
    /// * `String` - Complete base URL (e.g., "http://0.0.0.0:15205")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    ///
    /// let config = Config::default();
    /// let base_url = config.download_base_url();
    /// assert_eq!(base_url, "http://0.0.0.0:15205");
    /// ```
    pub fn download_base_url(&self) -> String {
        match &self.download_base_url {
            Some(url) => return url.clone(),
            None => {
                let protocol = if self.enable_https { "https" } else { "http" };
                format!(
                    "{}://{}:{}",
                    protocol, self.download_host, self.download_port
                )
            }
        }
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
        assert_eq!(config.admin_host, "127.0.0.1");
        assert_eq!(config.download_host, "0.0.0.0");
        assert!(!config.enable_https);
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
        assert_eq!(config.download_base_url(), "http://0.0.0.0:15205");

        config.enable_https = true;
        assert_eq!(config.download_base_url(), "https://0.0.0.0:15205");

        config.download_host = "example.com".to_string();
        config.download_port = 443;
        assert_eq!(config.download_base_url(), "https://example.com:443");
    }
}
