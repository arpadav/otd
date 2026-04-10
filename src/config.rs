//! Configuration management for the OTD server
//!
//! Ephemeral settings come from CLI args via [`clap`]
//! Persistent settings (password hash, download base URL) are saved to
//! `$XDG_DATA_HOME/otd/config.toml` and updated via the settings API
//!
//! The global [`CONFIG`] static holds a [`ParsedConfig`] behind a
//! [`tokio::sync::RwLock`], initialized once via [`init_config`]
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::OnceLock};
use tokio::sync::RwLock;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Default admin port to use if not specified via CLI
const DEFAULT_ADMIN_PORT: u16 = 15204;
/// Default admin host to use if not specified via CLI
const DEFAULT_ADMIN_HOST: &str = "127.0.0.1";
/// Default download port to use if not specified via CLI
const DEFAULT_DOWNLOAD_PORT: u16 = 15205;
/// Default download host to use if not specified via CLI
const DEFAULT_DOWNLOAD_HOST: &str = "0.0.0.0";
/// Environment variable name used to override the log level
pub(crate) const OTD_LOG_ENVIRONMENT_VAR: &str = "OTD_LOG";
/// Environment variable name used to override the log file path
pub(crate) const OTD_LOG_FILE_ENVIRONMENT_VAR: &str = "OTD_LOG_FILE";
/// Default log file name fallback
pub(crate) const OTD_LOG_FILE_DEFAULT_NAME: &str = "otd.log";

// --------------------------------------------------
// statics
// --------------------------------------------------
/// Global configuration, initialized once at startup via [`init_config`]
///
/// Accessed via [`config()`] which returns a reference to the inner [`RwLock`]
/// The [`RwLock`] is needed because `PUT /api/settings` mutates the persistent
/// fields at runtime
static CONFIG: OnceLock<RwLock<ParsedConfig>> = OnceLock::new();

#[inline(always)]
/// Returns a reference to the global config [`RwLock`]
///
/// # Panics
///
/// Panics if [`init_config`] has not been called yet
pub(crate) fn config() -> &'static RwLock<ParsedConfig> {
    #[allow(
        clippy::expect_used,
        reason = "init_config is always called at startup"
    )]
    CONFIG
        .get()
        .expect("CONFIG not initialized - call init_config() first")
}

/// Initializes the global [`CONFIG`] with the given CLI args
///
/// Loads [`PersistentConfig`] from disk and merges with CLI args
///
/// # Panics
///
/// Panics if called more than once
pub(crate) fn init_config(cli: CliConfig) {
    let parsed = ParsedConfig::from_cli(cli);
    // --------------------------------------------------
    // ensure persistent config file exists on disk so
    // users can discover its location
    // --------------------------------------------------
    if let Err(e) = parsed.persistent.save() {
        tracing::warn!("Failed to write initial config.toml: {e}");
    }
    #[allow(clippy::expect_used, reason = "init_config must only be called once")]
    CONFIG
        .set(RwLock::new(parsed))
        .expect("init_config called twice");
}

#[derive(Debug, thiserror::Error)]
/// Errors that can occur while reading or writing persistent config
pub enum ConfigError {
    #[error("I/O error: {0}")]
    /// Failed to read or write a file
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    /// Failed to serialize config to TOML
    Toml(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Parser)]
#[command(name = "otd", about = "One-time download link server")]
/// Ephemeral CLI configuration parsed from command-line arguments
///
/// These values are set at startup and do not change at runtime
pub(crate) struct CliConfig {
    /// Admin interface host
    #[arg(long, default_value = DEFAULT_ADMIN_HOST)]
    pub admin_host: String,

    /// Admin interface port
    #[arg(long, default_value_t = DEFAULT_ADMIN_PORT)]
    pub admin_port: u16,

    /// Download server host
    #[arg(long, default_value = DEFAULT_DOWNLOAD_HOST)]
    pub download_host: String,

    /// Download server port
    #[arg(long, default_value_t = DEFAULT_DOWNLOAD_PORT)]
    pub download_port: u16,

    /// Disable HTTPS (HTTPS is enabled by default)
    #[arg(long)]
    pub no_https: bool,

    /// Base directory for file serving (defaults to current directory)
    #[arg(long)]
    pub base_path: Option<String>,

    /// Log level: trace, debug, info, warn, error (overridden by OTD_LOG env)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Log file path (overridden by OTD_LOG_FILE env)
    #[arg(long)]
    pub log_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Persistent configuration saved to `data_dir()/config.toml`
///
/// Updated at runtime via the settings API and written atomically to disk
pub(crate) struct PersistentConfig {
    /// Argon2 hash of the admin password (`None` = no password required)
    pub admin_password_hash: Option<String>,
    /// Custom download base URL (`None` = derived from CLI host/port)
    pub download_base_url: Option<String>,
}
/// [`PersistentConfig`] implementation
impl PersistentConfig {
    /// Loads from `data_dir()/config.toml`, returning [`Default`] on missing file
    pub fn load_or_default() -> Self {
        let path = data_dir().join("config.toml");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Atomically writes to `data_dir()/config.toml` via `.tmp` rename
    pub fn save(&self) -> Result<(), ConfigError> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let tmp = dir.join("config.toml.tmp");
        let toml_str = toml::to_string_pretty(self).map_err(ConfigError::Toml)?;
        std::fs::write(&tmp, toml_str)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[derive(Debug)]
/// Pre-computed runtime configuration derived from [`CliConfig`] + [`PersistentConfig`]
///
/// Created once at startup via [`init_config`]. The [`PersistentConfig`] portion
/// may be mutated at runtime via the settings API
pub(crate) struct ParsedConfig {
    /// Ephemeral CLI configuration
    pub cli: CliConfig,
    /// Persistent on-disk configuration
    pub persistent: PersistentConfig,
    /// Pre-computed admin socket address
    pub admin_addr: SocketAddr,
    /// Pre-computed download socket address
    pub download_addr: SocketAddr,
    /// Resolved download base URL (persistent override or derived from CLI)
    pub download_base_url: String,
    /// Canonicalized base directory path for file serving
    pub canonical_base_path: PathBuf,
}
/// [`ParsedConfig`] implementation
impl ParsedConfig {
    /// Constructs a [`ParsedConfig`] from CLI args, loading persistent settings
    /// from disk
    ///
    /// Calls [`PersistentConfig::load_or_default`] to read `data_dir()/config.toml`,
    /// then delegates to [`Self::resolve`] to merge and pre-compute all runtime values
    ///
    /// # Arguments
    ///
    /// * `cli` - Parsed CLI arguments from [`CliConfig`]
    fn from_cli(cli: CliConfig) -> Self {
        let persistent = PersistentConfig::load_or_default();
        Self::resolve(cli, persistent)
    }

    /// Resolves CLI + persistent into final runtime values
    pub(crate) fn resolve(cli: CliConfig, persistent: PersistentConfig) -> Self {
        // --------------------------------------------------
        // parse socket addresses
        // --------------------------------------------------
        let admin_addr = format!("{}:{}", cli.admin_host, cli.admin_port)
            .parse()
            .unwrap_or_else(|e| {
                tracing::error!("Address parse error for admin: {e}. Defaulting.");
                #[allow(clippy::expect_used, reason = "default address is always valid")]
                format!("{DEFAULT_ADMIN_HOST}:{DEFAULT_ADMIN_PORT}")
                    .parse()
                    .expect("default address parse error")
            });
        let download_addr = format!("{}:{}", cli.download_host, cli.download_port)
            .parse()
            .unwrap_or_else(|e| {
                tracing::error!("Address parse error for download: {e}. Defaulting.");
                #[allow(clippy::expect_used, reason = "default address is always valid")]
                format!("{DEFAULT_DOWNLOAD_HOST}:{DEFAULT_DOWNLOAD_PORT}")
                    .parse()
                    .expect("default address parse error")
            });
        // --------------------------------------------------
        // resolve download base URL: persistent > derived from CLI
        // --------------------------------------------------
        let enable_https = !cli.no_https;
        let download_base_url = persistent.download_base_url.clone().unwrap_or_else(|| {
            let protocol = if enable_https { "https" } else { "http" };
            format!("{protocol}://{}:{}", cli.download_host, cli.download_port)
        });
        // --------------------------------------------------
        // resolve base path: CLI > cwd > /tmp
        // --------------------------------------------------
        let base_path_str = cli.base_path.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "/tmp".into())
        });
        let raw_path = PathBuf::from(&base_path_str);
        let canonical_base_path = std::fs::canonicalize(&raw_path).unwrap_or(raw_path);
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        ParsedConfig {
            cli,
            persistent,
            admin_addr,
            download_addr,
            download_base_url,
            canonical_base_path,
        }
    }

    /// Re-resolves `download_base_url` after a persistent config change
    ///
    /// Should be called after mutating [`ParsedConfig::persistent`] at runtime
    /// (e.g. after a `PUT /api/settings` request). Re-derives the URL from the
    /// persistent override or falls back to the CLI host/port with the appropriate
    /// `http`/`https` scheme
    pub(crate) fn refresh_download_base_url(&mut self) {
        let enable_https = !self.cli.no_https;
        self.download_base_url = self
            .persistent
            .download_base_url
            .clone()
            .unwrap_or_else(|| {
                let protocol = if enable_https { "https" } else { "http" };
                format!(
                    "{protocol}://{}:{}",
                    self.cli.download_host, self.cli.download_port
                )
            });
    }

    /// Safely joins `rel` onto [`Self::canonical_base_path`], blocking path
    /// traversal and symlink escapes via canonicalization
    ///
    /// Joins the relative path onto the canonical base, resolves symlinks and
    /// `..` segments via [`std::fs::canonicalize`], then verifies the result
    /// still starts with the base. Returns `None` if canonicalization fails
    /// (e.g. path does not exist) or if the resolved path escapes the base.
    /// Both failure cases log a warning
    ///
    /// # Arguments
    ///
    /// * `rel` - Relative path to join onto the base directory
    pub(crate) fn safe_join(&self, rel: &str) -> Option<PathBuf> {
        let joined = self.canonical_base_path.join(rel);
        std::fs::canonicalize(&joined)
            .inspect_err(|e| tracing::warn!("Failed to canonicalize '{rel}': {e}"))
            .ok()
            .filter(|c| {
                let safe = c.starts_with(&self.canonical_base_path);
                if !safe {
                    tracing::warn!(
                        "Path escape blocked: '{rel}' resolves to '{c:?}' outside base '{:?}'",
                        self.canonical_base_path
                    );
                }
                safe
            })
    }

    /// Builds a full download URL from a display name and token
    ///
    /// Produces a URL of the form `{download_base_url}/{encoded_name}?k={token}`.
    /// The name is percent-encoded via [`url_encode`] so spaces and special
    /// characters are safe for use in a URL path segment
    ///
    /// # Arguments
    ///
    /// * `name` - Display name of the file or archive (used as the URL path segment)
    /// * `token` - One-time download token appended as the `k` query parameter
    pub(crate) fn download_url(&self, name: &str, token: &str) -> String {
        format!(
            "{}/{}?k={}",
            self.download_base_url,
            url_encode(name),
            token
        )
    }
}

#[inline(always)]
/// URL-encodes a string for safe use in query parameters
///
/// Iterates over each byte of the input and percent-encodes anything that is
/// not an unreserved character (`A–Z`, `a–z`, `0–9`, `-`, `_`, `.`, `~`)
/// Spaces are encoded as `+` rather than `%20`. All other bytes are encoded
/// as `%XX` using uppercase hex digits
///
/// # Arguments
///
/// * `input` - The raw string to encode
///
/// # Example
///
/// ```rust,ignore
/// assert_eq!(url_encode("my file.zip"), "my+file.zip");
/// assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
/// ```
pub(crate) fn url_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    input
        .bytes()
        .fold(String::with_capacity(input.len()), |mut acc, b| {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    acc.push(b as char);
                }
                b' ' => acc.push('+'),
                _ => {
                    acc.push('%');
                    acc.push(HEX[(b >> 4) as usize] as char);
                    acc.push(HEX[(b & 0x0F) as usize] as char);
                }
            }
            acc
        })
}

#[inline]
/// Returns the application data directory for persistent state
///
/// Uses `$XDG_DATA_HOME/otd/` if set, otherwise `$HOME/.local/share/otd/`
pub(crate) fn data_dir() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        })
        .join("otd")
}

/// Initializes the tracing subscriber with configurable log level and optional file output
///
/// Builds an [`EnvFilter`] using the following priority order:
/// `OTD_LOG` environment variable > `--log-level` CLI flag > `"info"`
/// Always attaches a non-ANSI stdout layer. If `--log-file` or `OTD_LOG_FILE`
/// is set, also attaches a non-rolling file appender layer writing to that path
/// Both layers are registered with the global tracing registry
///
/// # Arguments
///
/// * `cli` - Parsed CLI config providing the log level and optional log file path
pub(crate) fn init_logging(cli: &CliConfig) {
    use std::str::FromStr;
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
    let log_level = cli.log_level.clone();
    let log_file = cli
        .log_file
        .clone()
        .or_else(|| std::env::var(OTD_LOG_FILE_ENVIRONMENT_VAR).ok());
    // --------------------------------------------------
    // build env filter: OTD_LOG env > CLI --log-level > default "info"
    // --------------------------------------------------
    let env_layer = match std::env::var(OTD_LOG_ENVIRONMENT_VAR) {
        Ok(val) => match EnvFilter::from_str(&val) {
            Ok(filter) => filter,
            Err(_) => EnvFilter::from_env(OTD_LOG_ENVIRONMENT_VAR),
        },
        Err(_) => EnvFilter::new(log_level),
    };
    // --------------------------------------------------
    // stdout layer (always present, no ANSI for portability)
    // --------------------------------------------------
    let ansi_layer = fmt::layer().with_ansi(false);
    // --------------------------------------------------
    // optional file layer
    // --------------------------------------------------
    let file_layer = log_file.as_ref().map(|log_file| {
        let path = std::path::Path::new(log_file);
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let filename = path
            .file_name()
            .unwrap_or(std::ffi::OsStr::new(OTD_LOG_FILE_DEFAULT_NAME));
        tracing_subscriber::fmt::layer()
            .with_writer(tracing_appender::rolling::never(dir, filename))
            .with_ansi(false)
    });
    // --------------------------------------------------
    // init
    // --------------------------------------------------
    tracing_subscriber::registry()
        .with(env_layer)
        .with(ansi_layer)
        .with(file_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cli_config() {
        let cli = CliConfig::parse_from(["otd"]);
        assert_eq!(cli.admin_port, 15204);
        assert_eq!(cli.download_port, 15205);
        assert_eq!(cli.admin_host, "127.0.0.1");
        assert_eq!(cli.download_host, "0.0.0.0");
        assert!(!cli.no_https);
        assert!(cli.base_path.is_none());
    }

    #[test]
    fn test_parsed_config_defaults() {
        let cli = CliConfig::parse_from(["otd"]);
        let persistent = PersistentConfig::default();
        let parsed = ParsedConfig::resolve(cli, persistent);
        assert_eq!(parsed.admin_addr.port(), 15204);
        assert_eq!(parsed.download_addr.port(), 15205);
        assert_eq!(parsed.download_base_url, "https://0.0.0.0:15205");
    }

    #[test]
    fn test_no_https_flag() {
        let cli = CliConfig::parse_from(["otd", "--no-https"]);
        let persistent = PersistentConfig::default();
        let parsed = ParsedConfig::resolve(cli, persistent);
        assert_eq!(parsed.download_base_url, "http://0.0.0.0:15205");
    }

    #[test]
    fn test_persistent_download_url_override() {
        let cli = CliConfig::parse_from(["otd"]);
        let persistent = PersistentConfig {
            admin_password_hash: None,
            download_base_url: Some("https://files.example.com".into()),
        };
        let parsed = ParsedConfig::resolve(cli, persistent);
        assert_eq!(parsed.download_base_url, "https://files.example.com");
    }

    #[test]
    fn test_persistent_config_roundtrip() {
        let cfg = PersistentConfig {
            admin_password_hash: Some("hash123".into()),
            download_base_url: Some("https://dl.example.com".into()),
        };
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let decoded: PersistentConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(decoded.admin_password_hash, cfg.admin_password_hash);
        assert_eq!(decoded.download_base_url, cfg.download_base_url);
    }

    #[test]
    fn test_persistent_config_default_is_empty() {
        let cfg = PersistentConfig::default();
        assert!(cfg.admin_password_hash.is_none());
        assert!(cfg.download_base_url.is_none());
    }

    #[test]
    fn test_data_dir() {
        let dir = data_dir();
        assert!(dir.to_string_lossy().ends_with("otd"));
    }

    #[test]
    fn test_url_encode_unreserved() {
        assert_eq!(url_encode("abc-123_test.zip"), "abc-123_test.zip");
    }

    #[test]
    fn test_url_encode_space() {
        assert_eq!(url_encode("my file.zip"), "my+file.zip");
    }

    #[test]
    fn test_url_encode_special() {
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn url_encode_unreserved_chars_pass_through() {
        assert_eq!(url_encode("abc-123_test.zip"), "abc-123_test.zip");
    }

    #[test]
    fn url_encode_space_becomes_plus() {
        assert_eq!(url_encode("my file.zip"), "my+file.zip");
    }

    #[test]
    fn url_encode_special_chars_are_percent_encoded() {
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn url_encode_empty_string() {
        assert_eq!(url_encode(""), "");
    }

    #[test]
    fn url_encode_unicode_bytes() {
        // "é" encodes as %C3%A9 in UTF-8
        assert_eq!(url_encode("é"), "%C3%A9");
    }

    #[test]
    fn test_refresh_download_base_url() {
        let cli = CliConfig::parse_from(["otd"]);
        let persistent = PersistentConfig::default();
        let mut parsed = ParsedConfig::resolve(cli, persistent);
        assert_eq!(parsed.download_base_url, "https://0.0.0.0:15205");

        parsed.persistent.download_base_url = Some("https://custom.url".into());
        parsed.refresh_download_base_url();
        assert_eq!(parsed.download_base_url, "https://custom.url");
    }
}
