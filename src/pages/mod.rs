//! Dioxus page components for the OTD admin panel
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod about;
mod browse;
mod dashboard;
mod layout;
mod links;
mod login;
pub(crate) mod styles;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use about::About;
pub(crate) use browse::Browse;
pub(crate) use dashboard::Dashboard;
pub(crate) use layout::Layout;
pub(crate) use links::Links;
pub(crate) use login::Login;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const NOT_FOUND_CENTER_STY: &str = crate::classes!(
    "flex",
    "flex-col",
    "items-center",
    "justify-center",
    "min-h-[60vh]"
);
const NOT_FOUND_TITLE_STY: &str = crate::classes!("text-4xl", "font-bold", "mb-4");

#[component]
/// 404 not found page component
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div { class: NOT_FOUND_CENTER_STY,
            h1 { class: NOT_FOUND_TITLE_STY, "404" }
            p { class: "text-text-muted", "Page not found" }
        }
    }
}
