//! Modal overlay component
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::svg::XIcon;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const MODAL_HEADER_STY: &str = crate::classes!("flex", "items-center", "justify-between", "mb-4");
const MODAL_TITLE_STY: &str = crate::classes!("text-lg", "font-semibold");
const MODAL_CLOSE_STY: &str = crate::classes!(
    "p-1",
    "rounded-md",
    "text-text-muted",
    "hover:text-text",
    "transition-colors",
    "cursor-pointer"
);

// --------------------------------------------------
// component
// --------------------------------------------------
#[component]
/// Reusable modal overlay with optional title and close button
pub fn Modal(
    show: bool,
    on_close: EventHandler<()>,
    #[props(default)] title: Option<String>,
    children: Element,
) -> Element {
    if !show {
        return rsx! {};
    }

    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-content",
                onclick: move |e| e.stop_propagation(),
                if let Some(t) = &title {
                    div { class: MODAL_HEADER_STY,
                        h2 { class: MODAL_TITLE_STY, "{t}" }
                        button {
                            class: MODAL_CLOSE_STY,
                            onclick: move |_| on_close.call(()),
                            XIcon {}
                        }
                    }
                }
                {children}
            }
        }
    }
}
