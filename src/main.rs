//! # One-Time Download (OTD) Server
//!
//! A lightweight, secure file sharing server that generates one-time download links
//! Built with Dioxus fullstack and Tailwind CSS
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod classes;
mod components;
#[cfg(feature = "server")]
mod config;
mod core;
mod pages;
mod requests;
mod routes;
#[cfg(feature = "server")]
mod state;
mod svg;

// --------------------------------------------------
// local
// --------------------------------------------------
#[cfg(feature = "server")]
use crate::{config::CONFIG, state::APP_STATE};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// OTD logo asset
const LOGO: Asset = asset!("/assets/logo.svg");
/// Compiled Tailwind CSS asset
const TAILWIND_OUTPUT: Asset = asset!("/assets/tailwind.css");

/// Entry point for the OTD web application
fn main() -> std::io::Result<()> {
    // --------------------------------------------------
    // launch server - admin panel + dl route
    // --------------------------------------------------
    #[cfg(feature = "server")]
    #[allow(clippy::unwrap_used, reason = "")]
    tokio::runtime::Runtime::new()?.block_on(launch_server())?;
    // --------------------------------------------------
    // wasm
    // --------------------------------------------------
    #[cfg(not(feature = "server"))]
    dioxus::launch(AdminApp);
    // --------------------------------------------------
    // return - this should never happen
    // --------------------------------------------------
    Ok(())
}

#[cfg(feature = "server")]
/// Launches the OTD admin panel and download route server
///
/// Binds two separate axum routers: one for admin (Dioxus fullstack)
/// and one for file downloads (plain axum)
async fn launch_server() -> std::io::Result<()> {
    // --------------------------------------------------
    // init logging
    // --------------------------------------------------
    config::init_logging();
    tracing::info!("Starting OTD admin panel");
    // --------------------------------------------------
    // init config
    // --------------------------------------------------
    std::sync::LazyLock::force(&CONFIG);
    // --------------------------------------------------
    // init shared state + background tasks
    // --------------------------------------------------
    std::sync::LazyLock::force(&APP_STATE);
    state::spawn_background_tasks();
    // --------------------------------------------------
    // get download and admin addr from config
    // --------------------------------------------------
    let (download_addr, admin_addr) = {
        let cfg = CONFIG.read().await;
        let download_addr = cfg.download_addr;
        let admin_addr = cfg.admin_addr;
        (download_addr, admin_addr)
    };
    // --------------------------------------------------
    // get download info, build and serve router
    // --------------------------------------------------
    let download_listener = tokio::net::TcpListener::bind(download_addr).await?;
    let download_router =
        axum::Router::new().fallback(axum::routing::get(crate::core::download::download_handler));
    tracing::info!("Download router listening on {download_addr}");
    // --------------------------------------------------
    // get admin info, build and serve router
    // --------------------------------------------------
    let admin_listener = tokio::net::TcpListener::bind(admin_addr).await?;
    let admin_router = axum::Router::new().serve_dioxus_application(ServeConfig::new(), AdminApp);
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

#[component]
/// Root Dioxus component for the admin panel
fn AdminApp() -> Element {
    rsx! {
        document::Title { "OTD - One-Time Downloads" }
        document::Link { rel: "icon", href: LOGO }
        document::Stylesheet { href: TAILWIND_OUTPUT }
        Router::<routes::AdminRoute> {}
    }
}
