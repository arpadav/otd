//! # One-Time Download (OTD) Server
//!
//! A lightweight, secure file sharing server that generates one-time download links.
//! Built with Rust and the `smol` async runtime for minimal resource usage.
//!
//! ## Features
//!
//! - **Dual-port architecture**: Separate admin interface and download server
//! - **One-time links**: Downloads expire after first use
//! - **File staging**: Select multiple files/folders before generating links
//! - **Security**: Path traversal protection and configurable access
//! - **Lightweight**: Minimal dependencies, built with `smol` async runtime
//! - **Configurable**: TOML-based configuration with environment variable overrides
//!
//! ## Quick Start
//!
//! ```bash
//! # Set base path and run
//! OTD_BASE_PATH=/path/to/files cargo run
//!
//! # Access admin interface
//! open http://localhost:15204
//!
//! # Download links will be served on
//! # http://localhost:15205/filename.ext?k=<uuid>
//! ```
//!
//! ## Architecture
//!
//! The server consists of several key components:
//!
//! - [`Config`]: Configuration management with TOML file support
//! - [`Server`]: Dual-port HTTP server using `smol` async runtime
//! - [`Handler`]: Request routing and business logic
//! - [`http::HttpResponse`]: HTTP response builder with proper headers
//! - [`AppState`]: Shared application state with download tokens
//!
//! ## Example
//!
//! ```rust,no_run
//! use otd::Server;
//! use smol_macros::main;
//! use macro_rules_attribute::apply;
//!
//! #[apply(main!)]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let server = Server::new().await;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod config;
pub mod handlers;
pub mod http;
pub mod server;
pub mod types;
// --------------------------------------------------
// prelude
// --------------------------------------------------
pub mod prelude {
    pub use crate::config::RwLockExt as _;
}
// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use config::{Config, ParsedConfig};
pub use handlers::Handler;
pub use server::Server;
pub use types::*;

/// Initializes the tracing subscriber with configurable log level and optional file output
///
/// This is called by [`crate::config::CONFIG`] upon its creation. Should not be called directly.
pub async fn init_logging() {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use crate::{
        config::{
            CONFIG, OTD_LOG_ENVIRONMENT_VAR, OTD_LOG_FILE_DEFAULT_NAME,
            OTD_LOG_FILE_ENVIRONMENT_VAR,
        },
        prelude::*,
    };
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use std::str::FromStr;
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let (log_level, log_file) = CONFIG
        .read_with(|cfg| {
            (
                cfg.raw.log_level.clone().unwrap_or(String::from("info")),
                cfg.raw.log_file.clone(),
            )
        })
        .await;
    let log_file = log_file.or_else(|| std::env::var(OTD_LOG_FILE_ENVIRONMENT_VAR).ok());
    // --------------------------------------------------
    // build env filter: OTD_LOG env > config log_level > default "info"
    // --------------------------------------------------
    let env_layer = match std::env::var(OTD_LOG_ENVIRONMENT_VAR) {
        Ok(val) => match EnvFilter::from_str(val.as_str()) {
            Ok(filter) => filter,
            Err(_) => EnvFilter::from_env(OTD_LOG_ENVIRONMENT_VAR),
        },
        Err(_) => EnvFilter::new(log_level),
    };
    // --------------------------------------------------
    // stdout layer (always present)
    // --------------------------------------------------
    // no ansi coloring, since some free bsd systems disallow
    // it by default. should make this configurable though
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
        // --------------------------------------------------
        // this will ALWAYS have ansi disabled, since file
        // --------------------------------------------------
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
