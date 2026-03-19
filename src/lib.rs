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
//! - [`HttpResponse`]: HTTP response builder with proper headers
//! - [`AppState`]: Shared application state with download tokens
//!
//! ## Example
//!
//! ```rust,no_run
//! use otd::{Config, Server};
//! use smol_macros::main;
//! use macro_rules_attribute::apply;
//!
//! #[apply(main!)]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let config = Config::load()?;
//!     let server = Server::new(config)?;
//!     server.run().await
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
// re-exports
// --------------------------------------------------
pub use config::{Config, ParsedConfig};
pub use handlers::Handler;
pub use server::Server;
pub use types::*;
