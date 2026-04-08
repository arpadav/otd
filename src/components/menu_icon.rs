//! Hamburger menu icon component
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const MENU_ICON_STY: &str = crate::classes!("w-5", "h-5");

// --------------------------------------------------
// component
// --------------------------------------------------
#[component]
/// Three-line hamburger menu icon for mobile navigation toggle
pub fn MenuIcon() -> Element {
    rsx! {
        svg {
            class: MENU_ICON_STY,
            fill: "none",
            view_box: "0 0 24 24",
            stroke_width: "2",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                d: "M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5",
            }
        }
    }
}
