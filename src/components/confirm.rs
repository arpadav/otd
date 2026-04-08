//! Confirmation dialog component
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::modal::Modal;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const CONFIRM_MSG_STY: &str = crate::classes!("text-sm", "text-text-muted", "mb-6");
const CONFIRM_ACTIONS_STY: &str = crate::classes!("flex", "items-center", "justify-end", "gap-3");

// --------------------------------------------------
// component
// --------------------------------------------------
#[component]
/// Modal confirmation dialog with confirm/cancel actions
pub fn ConfirmDialog(
    show: bool,
    title: String,
    message: String,
    #[props(default = "Confirm".to_string())] confirm_label: String,
    #[props(default = false)] danger: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        Modal {
            show,
            on_close: move |_| on_cancel.call(()),
            title: title.clone(),
            p { class: CONFIRM_MSG_STY, "{message}" }
            div { class: CONFIRM_ACTIONS_STY,
                button {
                    class: "btn",
                    onclick: move |_| on_cancel.call(()),
                    "Cancel"
                }
                button {
                    class: if danger { "btn-danger" } else { "btn-primary" },
                    onclick: move |_| on_confirm.call(()),
                    "{confirm_label}"
                }
            }
        }
    }
}
