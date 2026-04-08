//! Route definitions
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::pages::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

#[derive(Debug, Clone, Default, Routable, PartialEq)]
/// Admin routes, served on admin port
pub enum AdminRoute {
    #[route("/:..segments")]
    /// 404 page
    NotFound { segments: Vec<String> },

    #[default]
    #[layout(Layout)]
    #[route("/")]
    /// Admin dashboard
    Dashboard,

    #[route("/login")]
    /// Login page
    Login,

    #[route("/browse")]
    /// Browse items
    Browse,

    #[route("/links")]
    /// View and manage links
    Links,

    #[route("/about")]
    /// View about information
    About,
}
