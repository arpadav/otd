//! Spinner SVG icon for loading/preparing states
//!
//! Author: aav
use dioxus::prelude::*;

#[component]
pub(crate) fn SpinnerIcon(
    #[props(default = "w-3.5 h-3.5 animate-spin".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class: "{class}",
            fill: "none",
            view_box: "0 0 24 24",
            circle {
                class: "opacity-25",
                cx: "12",
                cy: "12",
                r: "10",
                stroke: "currentColor",
                stroke_width: "4",
            }
            path {
                class: "opacity-75",
                fill: "currentColor",
                d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z",
            }
        }
    }
}
