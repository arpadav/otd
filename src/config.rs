//! Configuration management for the OTD server.
//!
//! This module handles loading and managing server configuration from TOML files
//! and environment variables. It provides sensible defaults and automatic config
//! file generation.
//!
//! The global [`HOT_CONFIG`] static holds a [`ParsedConfig`] behind a
//! [`smol::lock::RwLock`]. A [`notify`]-based file watcher updates it on change.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use serde::{Deserialize, Serialize};
use smol::lock::RwLock;
use std::{net::SocketAddr, path::PathBuf, sync::LazyLock};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Default config file name to look for in the current directory or specified by the `OTD_CONFIG_FILE` environment variable.
const OTD_CONFIG_FILE: &str = "otd-config.toml";
/// Environment variable name used to specify the path to the config file.
const OTD_CONFIG_ENVIRONMENT_VAR: &str = "OTD_CONFIG_FILE";
/// Environment variable name used to specify the base path for the application.
const OTD_BASE_ENVIRONMENT_VAR: &str = "OTD_BASE_PATH";
/// Environment variable name used to specify the log level for the application.
pub const OTD_LOG_ENVIRONMENT_VAR: &str = "OTD_LOG";
/// Environment variable name used to specify the path to the log file for the application.
pub const OTD_LOG_FILE_ENVIRONMENT_VAR: &str = "OTD_LOG_FILE";
/// Default log file name to use if no path is specified in the config or environment.
///
/// Note that if both not defined in config OR env, this will not be used. Only used if
/// it is defined, but there is an error/issue with the parsing of the filename.
pub const OTD_LOG_FILE_DEFAULT_NAME: &str = "otd.log";
/// Default admin port to use if no port is specified in the config or environment.
const DEFAULT_ADMIN_PORT: u16 = 15204;
/// Default admin host to use if no host is specified in the config or environment.
const DEFAULT_ADMIN_HOST: &str = "127.0.0.1";
/// Default download port to use if no port is specified in the config or environment.
const DEFAULT_DOWNLOAD_PORT: u16 = 15205;
/// Default download host to use if no host is specified in the config or environment.
const DEFAULT_DOWNLOAD_HOST: &str = "0.0.0.0";
/// Default buffer size for reading HTTP requests.
const DEFAULT_BUFFER_SIZE: usize = 8192;
/// Default maximum request size in bytes.
const DEFAULT_MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB
/// Fallback base path to use if no base path is specified in the config or environment.
const DEFAULT_BASE_PATH_FALLBACK: &str = "/tmp";
/// Poll interval for the config file watcher.
const CONFIG_WATCH_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
/// Debounce window to prevent double-reloading on filesystems that emit duplicate events.
const CONFIG_RELOAD_DEBOUNCE: u128 = 10; // milliseconds
/// Timestamp of the last successful reload, used for debouncing.
static LAST_RELOAD: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// Global hot-reloadable configuration, initialized once at startup.
///
/// Accessed via `.read().await` in async handler contexts.
/// Updated by the [`notify`] config watcher via `smol::block_on(lock.write())`.
pub static CONFIG: LazyLock<RwLock<ParsedConfig>> = LazyLock::new(|| {
    // --------------------------------------------------
    // read in
    // --------------------------------------------------
    #[allow(clippy::expect_used, reason = "config file must exist at startup")]
    let (cfg, path) = Config::load().expect("failed to load config");
    // --------------------------------------------------
    // spawn a watcher for the path
    // --------------------------------------------------
    spawn_config_watcher(&path);
    RwLock::new(cfg.parse(path))
});

/// Spawns a [`notify`] file watcher for the config file.
///
/// The watcher is stored in a [`Box::leak`] to live for the process lifetime.
fn spawn_config_watcher(config_path: &PathBuf) {
    use notify::Watcher;
    let notify_config = notify::Config::default().with_poll_interval(CONFIG_WATCH_POLL_INTERVAL);
    match notify::RecommendedWatcher::new(ConfigWatcher, notify_config) {
        Ok(mut watcher) => {
            if let Err(e) = watcher.watch(config_path, notify::RecursiveMode::NonRecursive) {
                tracing::warn!("Failed to watch config file: {e}");
                return;
            }
            // --------------------------------------------------
            // leak the watcher so it lives for the process lifetime
            // --------------------------------------------------
            Box::leak(Box::new(watcher));
            tracing::info!("Config file watcher started for {config_path:?}");
        }
        Err(e) => tracing::warn!("Failed to create config watcher: {e}"),
    }
}
/// Watches the config file for changes and reloads [`CONFIG`] on modify.
struct ConfigWatcher;
/// [`ConfigWatcher`] implementation of [`notify::EventHandler`]
impl notify::EventHandler for ConfigWatcher {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(event) => {
                if let notify::EventKind::Modify(_) = event.kind {
                    if let Ok(mut guard) = LAST_RELOAD.lock() {
                        if let Some(instant) = guard.as_ref()
                            && instant.elapsed().as_millis() < CONFIG_RELOAD_DEBOUNCE
                        {
                            return;
                        }
                        *guard = Some(std::time::Instant::now());
                    }
                    let cfg_path = smol::block_on(CONFIG.read_with(|cfg| cfg.path.clone()));
                    match Config::load_from_path(&cfg_path) {
                        Ok(new_cfg) => {
                            smol::block_on(
                                CONFIG.write_with(|cfg| *cfg = ParsedConfig::from(new_cfg)),
                            );
                            tracing::info!("Config reloaded successfully");
                        }
                        Err(e) => tracing::warn!("Config reload failed (keeping old config): {e}"),
                    }
                }
            }
            Err(e) => tracing::error!("Config watcher error: {e}"),
        }
    }
}

/// [`RwLockExt`] provides an async `.read_with()`
pub trait RwLockExt<T> {
    fn read_with<F, R>(&self, f: F) -> impl Future<Output = R>
    where
        F: FnOnce(&T) -> R;

    fn write_with<F, R>(&self, f: F) -> impl Future<Output = R>
    where
        F: FnOnce(&mut T) -> R;
}
/// [`smol::lock::RwLock`] implementation of [`RwLockExt`]
impl<T> RwLockExt<T> for RwLock<T> {
    async fn read_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let lock = self.read().await;
        f(&*lock)
    }

    async fn write_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut lock = self.write().await;
        f(&mut *lock)
    }
}

#[derive(Debug, thiserror::Error)]
/// Represents errors that can occur while reading or writing the config file
pub enum ConfigError {
    #[error("Failed to read/write config file: {0}")]
    /// Failed to read/write the config file
    Io(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    /// Failed to parse the config file
    ReadToml(#[from] toml::de::Error),

    #[error("Failed to write config file: {0}")]
    /// Failed to write the config file - for a default
    /// when it does not exist
    WriteToml(#[from] toml::ser::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
/// let (config, _path) = Config::load().unwrap();
/// println!("Admin port: {}", config.admin_port);
/// ```
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
    /// Base url for download links (e.g., `https://files.example.com/`)
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
    /// Log level filter: "trace", "debug", "info", "warn", or "error".
    /// Can be overridden by the `OTD_LOG` environment variable.
    /// Defaults to "info" when not set.
    pub log_level: Option<String>,
    /// Optional log file path. When set, logs are written to this file
    /// in addition to stdout. The parent directory must exist.
    pub log_file: Option<String>,
}
/// [`Config`] implementation of [`Default`]
impl Default for Config {
    fn default() -> Self {
        Self {
            admin_port: DEFAULT_ADMIN_PORT,
            admin_host: DEFAULT_ADMIN_HOST.into(),
            download_port: DEFAULT_DOWNLOAD_PORT,
            download_host: DEFAULT_DOWNLOAD_HOST.into(),
            download_base_url: None,
            base_path: std::env::current_dir()
                .map(|p| p.to_string_lossy().into())
                .unwrap_or_else(|_| DEFAULT_BASE_PATH_FALLBACK.into()),
            buffer_size: DEFAULT_BUFFER_SIZE,
            max_request_size: DEFAULT_MAX_REQUEST_SIZE,
            enable_https: false,
            cert_path: None,
            key_path: None,
            admin_token: None,
            admin_password: None,
            log_level: None,
            log_file: None,
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
    ///
    /// - `OTD_BASE_PATH`: Overrides the base_path setting
    /// - `OTD_LOG`: Overrides the log_level setting
    ///
    /// # Returns
    ///
    /// A tuple of the loaded `Config` and the `PathBuf` of the config file used.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use otd::Config;
    ///
    /// let (config, path) = Config::load().unwrap();
    /// println!("Base path: {}", config.base_path);
    /// ```
    pub fn load() -> Result<(Self, PathBuf), ConfigError> {
        // --------------------------------------------------
        // create default file if not exists
        // --------------------------------------------------
        let path = match (
            PathBuf::from(OTD_CONFIG_FILE).exists(),
            std::env::var(OTD_CONFIG_ENVIRONMENT_VAR),
        ) {
            (_, Ok(cfg_path)) => {
                tracing::info!(
                    "Using cfg file from environment variable {OTD_CONFIG_ENVIRONMENT_VAR}: {cfg_path}"
                );
                PathBuf::from(cfg_path)
            }
            (false, Err(_)) => {
                let default_cfg = Self::default();
                let toml_str =
                    toml::to_string_pretty(&default_cfg).map_err(ConfigError::WriteToml)?;
                std::fs::write(OTD_CONFIG_FILE, toml_str)?;
                tracing::info!("Created default cfg file: {OTD_CONFIG_FILE}");
                PathBuf::from(OTD_CONFIG_FILE)
            }
            (true, Err(_)) => {
                tracing::warn!(
                    "Failed to read config file from environment variable {OTD_CONFIG_ENVIRONMENT_VAR}, using default"
                );
                PathBuf::from(OTD_CONFIG_FILE)
            }
        };
        let mut cfg = Self::load_from_path(&path)?;
        // --------------------------------------------------
        // override with environment variables if present
        // --------------------------------------------------
        if let Ok(base_path) = std::env::var(OTD_BASE_ENVIRONMENT_VAR) {
            cfg.base_path = base_path;
        }
        if let Ok(log_level) = std::env::var(OTD_LOG_ENVIRONMENT_VAR) {
            cfg.log_level = Some(log_level);
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok((cfg, path))
    }

    /// Loads configuration from a specific file path without side effects.
    ///
    /// Unlike [`Config::load`], this does not create default config files or
    /// apply environment variable overrides. Used by the hot-reload task.
    // fn load_from_path(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
    fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        // --------------------------------------------------
        // read the config file using toml
        // --------------------------------------------------
        let config_str = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config: Config = toml::from_str(&config_str).map_err(ConfigError::ReadToml)?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(config)
    }

    #[inline(always)]
    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Resolves a [`Config`] into a [`ParsedConfig`] by pre-computing
    /// socket addresses and the download base URL.
    ///
    /// Uses a default config path. For specifying the config file location,
    /// use [`Config::parse_with_path`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    /// use std::path::PathBuf;
    ///
    /// let parsed = Config::default().parse(Default::default());
    /// assert_eq!(parsed.admin_addr.port(), 15204);
    /// ```
    pub(crate) fn parse(self, path: PathBuf) -> ParsedConfig {
        let mut parsed = ParsedConfig::from(self);
        parsed.path = path;
        parsed
    }
}

#[derive(Debug)]
/// Pre-computed configuration derived from [`Config`].
///
/// Created at startup via [`Config::parse`] and stored in the global
/// [`HOT_CONFIG`] static. All values that would otherwise be recomputed
/// per-request (socket addresses, download base URL, canonicalized base path)
/// are resolved here.
///
/// Also shared behind an `Arc` in the [`Handler`](crate::handlers::Handler)
/// for the startup snapshot (used for template rendering).
///
/// # Examples
///
/// ```rust
/// use otd::Config;
/// use std::path::PathBuf;
///
/// let parsed = Config::default().parse(Default::default());
/// assert_eq!(parsed.admin_addr.port(), 15204);
/// assert_eq!(parsed.download_base_url, "http://0.0.0.0:15205");
/// ```
pub struct ParsedConfig {
    /// The original TOML-level configuration.
    pub raw: Config,
    /// Path to the configuration file.
    pub path: PathBuf,
    /// Pre-computed admin socket address.
    pub admin_addr: SocketAddr,
    /// Pre-computed download socket address.
    pub download_addr: SocketAddr,
    /// Pre-computed download base URL (e.g., "http://0.0.0.0:15205").
    pub download_base_url: String,
    /// Canonicalized base directory path for file serving.
    pub canonical_base_path: PathBuf,
}
/// [`ParsedConfig`] implementation of [`From`] for [`Config`]
///
/// Please note that the `path` field will be set to [`PathBuf::new()`] by default,
/// so it is recommended to use [`Config::parse`] instead.
impl From<Config> for ParsedConfig {
    fn from(cfg: Config) -> Self {
        let admin_addr = Self::admin_addr(&cfg).unwrap_or_else(|e| {
            tracing::error!("Address parse error for admin address. Defaulting to {DEFAULT_ADMIN_HOST}:{DEFAULT_ADMIN_PORT}: {e}");
            #[allow(clippy::expect_used, reason = "default address is always valid")]
            format!("{DEFAULT_ADMIN_HOST}:{DEFAULT_ADMIN_PORT}").parse().expect("This should never panic - default address is always valid, contact OTD developer")
        });
        let download_addr = Self::download_addr(&cfg).unwrap_or_else(|e| {
            tracing::error!(
                "Address parse error for download address. Defaulting to 0.0.0.0:{DEFAULT_DOWNLOAD_PORT}: {e}"
            );
            #[allow(clippy::expect_used, reason = "default address is always valid")]
            format!("{DEFAULT_DOWNLOAD_HOST}:{DEFAULT_DOWNLOAD_PORT}").parse().expect("This should never panic - default address is always valid, contact OTD developer")
        });
        let download_base_url = Self::download_base_url(&cfg);
        let raw_path = PathBuf::from(&cfg.base_path);
        let canonical_base_path = std::fs::canonicalize(&raw_path).unwrap_or(raw_path);
        ParsedConfig {
            path: PathBuf::new(),
            raw: cfg,
            admin_addr,
            download_addr,
            download_base_url,
            canonical_base_path,
        }
    }
}
/// [`ParsedConfig`] implementation
impl ParsedConfig {
    #[inline(always)]
    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Returns the socket address for the admin interface.
    ///
    /// Combines the admin host and port into a `SocketAddr` for binding.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    /// use otd::config::ParsedConfig;
    ///
    /// let config = Config::default();
    /// let addr = ParsedConfig::admin_addr(&config).unwrap();
    /// assert_eq!(addr.port(), 15204);
    /// ```
    fn admin_addr(cfg: &Config) -> Result<SocketAddr, std::net::AddrParseError> {
        format!("{}:{}", cfg.admin_host, cfg.admin_port).parse()
    }

    #[inline(always)]
    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Returns the socket address for the download server.
    ///
    /// Combines the download host and port into a `SocketAddr` for binding.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    /// use otd::config::ParsedConfig;
    ///
    /// let config = Config::default();
    /// let addr = ParsedConfig::download_addr(&config).unwrap();
    /// assert_eq!(addr.port(), 15205);
    /// ```
    fn download_addr(cfg: &Config) -> Result<SocketAddr, std::net::AddrParseError> {
        format!("{}:{}", cfg.download_host, cfg.download_port).parse()
    }

    #[cfg_attr(feature = "doc-tests", visibility::make(pub))]
    /// Returns the base URL for download links.
    ///
    /// Constructs the complete base URL including protocol, host, and port
    /// for generating download links. Uses HTTPS if enabled, HTTP otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use otd::Config;
    /// use otd::config::ParsedConfig;
    ///
    /// let config = Config::default();
    /// let base_url = ParsedConfig::download_base_url(&config);
    /// assert_eq!(base_url, "http://0.0.0.0:15205");
    /// ```
    fn download_base_url(cfg: &Config) -> String {
        match &cfg.download_base_url {
            Some(url) => url.clone(),
            None => {
                let protocol: &'static str = ["http", "https"][cfg.enable_https as usize];
                format!("{}://{}:{}", protocol, cfg.download_host, cfg.download_port)
            }
        }
    }
}

#[inline]
/// Returns the application data directory for persistent state.
///
/// Uses `$XDG_DATA_HOME/otd/` if set, otherwise `$HOME/.local/share/otd/`.
pub(crate) fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        })
        .join("otd")
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
        let admin_addr = ParsedConfig::admin_addr(&Config::default()).unwrap();
        assert_eq!(admin_addr.port(), 15204);

        let download_addr = ParsedConfig::download_addr(&Config::default()).unwrap();
        assert_eq!(download_addr.port(), 15205);
    }

    #[test]
    fn test_download_base_url() {
        let mut cfg = Config::default();
        let download_base_url = ParsedConfig::download_base_url(&cfg);
        assert_eq!(download_base_url, "http://0.0.0.0:15205".to_string());
        cfg.enable_https = true;
        let download_base_url = ParsedConfig::download_base_url(&cfg);
        assert_eq!(download_base_url, "https://0.0.0.0:15205".to_string());

        cfg.download_host = "example.com".to_string();
        cfg.download_port = 443;
        let download_base_url = ParsedConfig::download_base_url(&cfg);
        assert_eq!(download_base_url, "https://example.com:443".to_string());
    }

    #[test]
    fn test_parsed_config() {
        let parsed = Config::default().parse(Default::default());
        assert_eq!(parsed.admin_addr.port(), 15204);
        assert_eq!(parsed.download_addr.port(), 15205);
        assert_eq!(parsed.download_base_url, "http://0.0.0.0:15205");
        assert_eq!(parsed.raw.admin_host, "127.0.0.1");
    }

    #[test]
    fn test_parsed_config_carries_auth() {
        let config = Config {
            admin_token: Some("secret".into()),
            admin_password: Some("pass".into()),
            ..Default::default()
        };
        let parsed = config.parse(Default::default());
        assert_eq!(parsed.raw.admin_token.as_deref(), Some("secret"));
        assert_eq!(parsed.raw.admin_password.as_deref(), Some("pass"));
    }

    #[test]
    fn test_canonical_base_path() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            base_path: dir.path().to_string_lossy().into(),
            ..Default::default()
        };
        let parsed = config.parse(Default::default());
        assert_eq!(
            parsed.canonical_base_path,
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn test_load_from_path_invalid() {
        let result = Config::load_from_path(&PathBuf::from("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_path_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-config.toml");
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, toml_str).unwrap();

        let loaded = Config::load_from_path(&path).unwrap();
        assert_eq!(loaded.admin_port, 15204);
    }

    #[test]
    fn test_data_dir() {
        let dir = data_dir();
        assert!(dir.to_string_lossy().ends_with("otd"));
    }
}
