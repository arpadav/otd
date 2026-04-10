//! # One-Time Download (OTD) Server
//!
//! A lightweight, secure file sharing server that generates one-time download links
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod api;
mod archive;
mod auth;
mod config;
mod download;
mod embed;
mod generated;
mod health;
mod state;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use generated::shared;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::state::APP_STATE;

/// Entry point for the OTD server
///
/// Parses CLI arguments, initializes logging, warns about deprecated config files,
/// initializes the global config, and blocks on the async server runtime
fn main() -> std::io::Result<()> {
    // --------------------------------------------------
    // parse CLI args and initialize logging
    // --------------------------------------------------
    let cli = <config::CliConfig as clap::Parser>::parse();
    config::init_logging(&cli);
    // --------------------------------------------------
    // initialize global config from parsed CLI args
    // --------------------------------------------------
    config::init_config(cli);
    // --------------------------------------------------
    // block on the async server until it exits
    // --------------------------------------------------
    tokio::runtime::Runtime::new()?.block_on(launch_server())
}

/// Launches the OTD server with separate admin and download routers
///
/// Initializes shared application state and background tasks, then binds two
/// independent [`axum`] routers on their respective addresses: the download
/// router handles all file download requests via a catch-all GET handler, and
/// the admin router serves the embedded SPA frontend and the REST API. Both
/// routers are served concurrently via [`tokio::try_join!`]
async fn launch_server() -> std::io::Result<()> {
    tracing::info!("Starting OTD server");
    // --------------------------------------------------
    // init shared state + background tasks
    // --------------------------------------------------
    std::sync::LazyLock::force(&APP_STATE);
    state::spawn_background_tasks();
    // --------------------------------------------------
    // get download and admin addr from config
    // --------------------------------------------------
    let (download_addr, admin_addr) = {
        let cfg = config::config().read().await;
        (cfg.download_addr, cfg.admin_addr)
    };
    // --------------------------------------------------
    // get download info, build and serve router
    // --------------------------------------------------
    let download_listener = tokio::net::TcpListener::bind(download_addr).await?;
    let download_router =
        axum::Router::new().fallback(axum::routing::get(crate::download::download_handler));
    tracing::info!("Download router listening on {download_addr}");
    // --------------------------------------------------
    // get admin info, build and serve router
    // --------------------------------------------------
    let admin_listener = tokio::net::TcpListener::bind(admin_addr).await?;
    let admin_router = axum::Router::new()
        .nest("/api", api::router())
        .fallback(axum::routing::get(embed::spa_handler));
    tracing::info!("Admin panel listening on {admin_addr}");
    // --------------------------------------------------
    // serve both routers concurrently
    // --------------------------------------------------
    tokio::try_join!(
        axum::serve(download_listener, download_router),
        axum::serve(admin_listener, admin_router),
    )?;
    // --------------------------------------------------
    // return ok - should never happen
    // --------------------------------------------------
    Ok(())
}
