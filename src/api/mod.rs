//! Axum JSON API router
//!
//! All admin panel API endpoints are defined here. Handler implementations
//! live in [`routes`] submodules
//!
//! Author: aav

// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod error;
pub mod routes;

// --------------------------------------------------
// external
// --------------------------------------------------
use axum::Router;
use axum::routing::{delete, get, post, put};

/// Builds the API router with all endpoints
///
/// Registers every handler under the `/api` prefix (applied by the caller
/// when this router is nested into the main application router). All routes
/// are protected by [`crate::auth::middleware`] which is applied as a layer
/// after all route registrations, covering every endpoint in this router
pub fn router() -> Router {
    Router::new()
        // --------------------------------------------------
        // auth
        // --------------------------------------------------
        .route("/auth/login", post(crate::auth::login))
        .route("/auth/logout", post(crate::auth::logout))
        // --------------------------------------------------
        // theme
        // --------------------------------------------------
        .route("/theme", get(routes::theme::get_theme))
        .route("/theme", put(routes::theme::set_theme))
        // --------------------------------------------------
        // stats + browse + links
        // --------------------------------------------------
        .route("/stats", get(routes::links::stats))
        .route("/browse", get(routes::browse::browse))
        .route("/links", get(routes::links::list_links))
        .route("/links", post(routes::links::generate_link))
        .route(
            "/links/{token}",
            delete(routes::links::delete_link).put(routes::links::edit_link),
        )
        .route("/links/{token}/revive", post(routes::links::revive_link))
        .route("/links", delete(routes::links::bulk_delete_links))
        // --------------------------------------------------
        // settings
        // --------------------------------------------------
        .route("/settings", get(routes::settings::get_settings))
        .route("/settings", put(routes::settings::update_settings))
        .route(
            "/settings/password",
            post(routes::settings::change_password),
        )
        // --------------------------------------------------
        // auth middleware
        // --------------------------------------------------
        .layer(axum::middleware::from_fn(crate::auth::middleware))
}
