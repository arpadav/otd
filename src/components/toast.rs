//! Toast notification system
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
// constants
// --------------------------------------------------
/// Auto-dismiss duration in milliseconds
const TOAST_TTL_MS: u64 = 4_000;

// --------------------------------------------------
// style
// --------------------------------------------------
const TOAST_DISMISS_STY: &str = crate::classes!(
    "ml-auto",
    "p-0.5",
    "rounded",
    "text-text-muted",
    "hover:text-text",
    "transition-colors",
    "cursor-pointer",
    "shrink-0"
);
const TOAST_ACTION_STY: &str = crate::classes!(
    "text-xs",
    "font-medium",
    "text-accent",
    "hover:text-accent-hover",
    "transition-colors",
    "cursor-pointer"
);

#[derive(Clone, Debug, PartialEq)]
/// Visual style variant for a toast notification
pub enum ToastVariant {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
/// Optional action button attached to a toast
pub struct ToastAction {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
/// A single toast notification entry
pub struct ToastItem {
    pub id: u32,
    pub message: String,
    pub variant: ToastVariant,
    pub action: Option<ToastAction>,
}

/// Signal containing the active toast list
pub type Toasts = Signal<Vec<ToastItem>>;

/// Handle for pushing toast notifications, obtained via [`use_toast`]
#[derive(Clone, Copy)]
pub struct Toast {
    toasts: Toasts,
}

/// [`Toast`] implementation
impl Toast {
    /// Pushes a toast notification that auto-dismisses after 4 seconds
    pub fn push(
        &self,
        message: impl Into<String>,
        variant: ToastVariant,
        action: Option<ToastAction>,
    ) {
        let mut toasts = self.toasts;
        let id = toasts.read().last().map(|t| t.id + 1).unwrap_or(0);
        toasts.write().push(ToastItem {
            id,
            message: message.into(),
            variant,
            action,
        });
        // --------------------------------------------------
        // auto-dismiss after TTL
        // --------------------------------------------------
        spawn(async move {
            #[cfg(feature = "web")]
            gloo_timers::future::TimeoutFuture::new(TOAST_TTL_MS as u32).await;
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(TOAST_TTL_MS)).await;
            toasts.write().retain(|t| t.id != id);
        });
    }
}

/// Returns a [`Toast`] handle for pushing notifications
pub fn use_toast() -> Toast {
    Toast {
        toasts: use_context(),
    }
}

#[component]
/// Renders active toast notifications in a fixed-position container
pub fn ToastProvider() -> Element {
    let toasts: Toasts = use_context();
    let items = toasts.read().clone();

    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "toast-container",
            for item in items.iter() {
                {
                    let variant_class = match item.variant {
                        ToastVariant::Success => "toast toast-success",
                        ToastVariant::Error => "toast toast-error",
                        ToastVariant::Info => "toast toast-info",
                    };
                    let id = item.id;
                    rsx! {
                        div { key: "{id}", class: "{variant_class}",
                            span { "{item.message}" }
                            if let Some(action) = &item.action {
                                button {
                                    class: TOAST_ACTION_STY,
                                    onclick: {
                                        let url = action.url.clone();
                                        move |_| {
                                            let url = url.clone();
                                            document::eval(&format!(
                                                "navigator.clipboard.writeText('{url}')"
                                            ));
                                        }
                                    },
                                    "{action.label}"
                                }
                            }
                            button {
                                class: TOAST_DISMISS_STY,
                                onclick: {
                                    let mut toasts = toasts;
                                    move |_| {
                                        toasts.write().retain(|t| t.id != id);
                                    }
                                },
                                XIcon { class: "w-3 h-3".to_string() }
                            }
                        }
                    }
                }
            }
        }
    }
}
