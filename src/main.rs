//! A simple file server for sharing files over the local network.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use otd::{Config, Server};

/// Initializes the tracing subscriber with configurable log level and optional file output.
fn init_logging(cfg: &Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // --------------------------------------------------
    // build env filter: OTD_LOG env > config log_level > default "info"
    // --------------------------------------------------
    let filter = if std::env::var("OTD_LOG").is_ok() {
        EnvFilter::from_env("OTD_LOG")
    } else {
        let level = cfg.log_level.as_deref().unwrap_or("info");
        EnvFilter::new(level)
    };

    // --------------------------------------------------
    // stdout layer (always present)
    // --------------------------------------------------
    let stdout_layer = fmt::layer().with_ansi(false);

    // --------------------------------------------------
    // optional file layer
    // --------------------------------------------------
    match &cfg.log_file {
        Some(log_file) => {
            let path = std::path::Path::new(log_file);
            let dir = path.parent().unwrap_or(std::path::Path::new("."));
            let filename = path
                .file_name()
                .ok_or("log_file has no filename component")?;
            let file_appender = tracing_appender::rolling::never(dir, filename);
            let file_layer = fmt::layer().with_ansi(false).with_writer(file_appender);
            tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .with(file_layer)
                .init();
        }
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .init();
        }
    }
    Ok(())
}

/// Initializes logging, config, etc.
fn init() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // --------------------------------------------------
    // get config (before logging init — tracing silently drops
    // events when no subscriber is installed yet)
    // --------------------------------------------------
    let cfg = Config::load()?;
    // --------------------------------------------------
    // init logging
    // --------------------------------------------------
    init_logging(&cfg)?;
    Ok(cfg)
}

/// The main entry point of the application.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // --------------------------------------------------
    // init and get cfg
    // --------------------------------------------------
    let cfg = init()?;
    let admin_host = &cfg.admin_host;
    let admin_port = cfg.admin_port;
    let download_host = &cfg.download_host;
    let download_port = cfg.download_port;
    let base_path = &cfg.base_path;
    let enable_https = cfg.enable_https;
    tracing::info!("Configuration loaded:");
    tracing::info!("  Admin server: {admin_host}:{admin_port}");
    tracing::info!("  Download server: {download_host}:{download_port}");
    tracing::info!("  Base path: {base_path}");
    tracing::info!("  HTTPS enabled: {enable_https}");
    let server = Server::new(cfg)?;
    smol::block_on(server.run())
}
