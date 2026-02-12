use otd::{Config, Server};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let config = Config::load()?;
    tracing::info!("Configuration loaded:");
    tracing::info!(
        "  Admin server: {}:{}",
        config.admin_host,
        config.admin_port
    );
    tracing::info!(
        "  Download server: {}:{}",
        config.download_host,
        config.download_port
    );
    tracing::info!("  Base path: {}", config.base_path);
    tracing::info!("  HTTPS enabled: {}", config.enable_https);
    // ---
    let server = Server::new(config);
    smol::block_on(server.run())
}
