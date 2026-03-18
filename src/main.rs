//! A simple file server for sharing files over the local network.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use otd::{Config, Server};

/// Initializes logging, config, etc.
fn init() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // --------------------------------------------------
    // init logging
    // --------------------------------------------------
    tracing_subscriber::fmt::init();
    // --------------------------------------------------
    // get config
    // --------------------------------------------------
    Config::load()
}

/// The main entry point of the application.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // --------------------------------------------------
    // init and get cfg
    // --------------------------------------------------
    let cfg = init()?;
    tracing::info!("Configuration loaded:");
    tracing::info!("  Admin server: {}:{}", cfg.admin_host, cfg.admin_port);
    tracing::info!(
        "  Download server: {}:{}",
        cfg.download_host,
        cfg.download_port
    );
    tracing::info!("  Base path: {}", cfg.base_path);
    tracing::info!("  HTTPS enabled: {}", cfg.enable_https);
    let server = Server::new(cfg);
    smol::block_on(server.run())
}
