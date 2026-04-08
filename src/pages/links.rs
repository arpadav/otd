//! Links management page
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::components::{ConfirmDialog, LinkRow, ToastVariant, use_toast};
use crate::core::links;
use crate::pages::styles::{EMPTY_STATE_STY, ERROR_STATE_STY, LOADING_STATE_STY, PAGE_TITLE_STY};
use crate::svg::SpinnerIcon;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const HEADER_ROW_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "justify-between",
    "mb-6",
    "flex-wrap",
    "gap-3"
);
const HEADER_ACTIONS_STY: &str = crate::classes!("flex", "items-center", "gap-2");
const BTN_XS_STY: &str = "btn-xs";
const BTN_XS_DANGER_STY: &str = "btn-xs-danger";
const CARD_TABLE_STY: &str = crate::classes!("card", "overflow-x-auto");
const TABLE_HEADER_BORDER_STY: &str = crate::classes!("border-b", "border-border");
const TABLE_HEADER_STY: &str = crate::classes!(
    "px-4",
    "py-2",
    "text-left",
    "text-xs",
    "font-medium",
    "text-text-muted",
    "uppercase",
    "tracking-wider"
);
const SUMMARY_BAR_STY: &str = crate::classes!(
    "text-sm",
    "text-text-muted",
    "mb-4",
    "flex",
    "items-center",
    "gap-1",
    "flex-wrap"
);
const REFRESH_SPIN_STY: &str = crate::classes!("inline-flex", "items-center", "gap-1");

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Polling interval in ms: 2s while any archive is preparing, 30s otherwise
const POLL_FAST_MS: u32 = 2_000;
/// Polling interval in ms: 30s while no archive is preparing
const POLL_SLOW_MS: u32 = 30_000;

#[allow(
    clippy::enum_variant_names,
    reason = "might add more actions that arent Remove"
)]
#[derive(Clone, PartialEq)]
/// Bulk action variants for batch link removal
enum BulkAction {
    /// Remove all links that have reached their download limit
    RemoveUsed,
    /// Remove all links that have passed their expiry time
    RemoveExpired,
    /// Remove all links unconditionally
    RemoveAll,
}

/// [`BulkAction`] implementation
impl BulkAction {
    /// Returns the dialog title for this bulk action
    fn title(&self) -> &'static str {
        match self {
            Self::RemoveUsed => "Remove Used Links",
            Self::RemoveExpired => "Remove Expired Links",
            Self::RemoveAll => "Remove All Links",
        }
    }
    /// Returns the confirmation message, interpolating the total link count where relevant
    fn message(&self, count: usize) -> String {
        match self {
            Self::RemoveUsed => "Remove all used links? This cannot be undone.".into(),
            Self::RemoveExpired => "Remove all expired links? This cannot be undone.".into(),
            Self::RemoveAll => format!("Remove all {count} links? This cannot be undone."),
        }
    }
    /// Returns the confirm button label for this bulk action
    fn label(&self) -> &'static str {
        match self {
            Self::RemoveUsed => "Remove Used",
            Self::RemoveExpired => "Remove Expired",
            Self::RemoveAll => "Remove All",
        }
    }
    /// Returns the API filter key sent to the bulk-delete endpoint
    fn api_key(&self) -> &'static str {
        match self {
            Self::RemoveUsed => "used",
            Self::RemoveExpired => "expired",
            Self::RemoveAll => "all",
        }
    }
}

#[component]
/// Links management page with table listing, bulk actions, and delete dialogs
pub fn Links() -> Element {
    let mut links = use_resource(move || async move { links::list_links().await });
    let mut deleting = use_signal(|| false);
    let mut pending_bulk = use_signal(|| None::<BulkAction>);
    let mut pending_delete = use_signal(|| None::<String>);
    let toast = use_toast();

    // --------------------------------------------------
    // adaptive polling: faster while any archive is preparing,
    // slower while all archives are ready or no links exist
    // --------------------------------------------------
    use_future(move || async move {
        loop {
            let has_preparing = links
                .read()
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .is_some_and(|v| v.iter().any(|i| i.archive_status == "preparing"));
            let ms = if has_preparing {
                POLL_FAST_MS
            } else {
                POLL_SLOW_MS
            };
            #[cfg(feature = "web")]
            gloo_timers::future::TimeoutFuture::new(ms).await;
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
            links.restart();
        }
    });

    // --------------------------------------------------
    // compute summary counts
    // --------------------------------------------------
    let (total, active, used, expired, preparing) = links
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|items| {
            let mut active = 0usize;
            let mut used = 0usize;
            let mut expired = 0usize;
            let mut preparing = 0usize;
            for i in items {
                if i.archive_status == "preparing" {
                    preparing += 1;
                } else if i.expired {
                    expired += 1;
                } else if i.remaining_downloads == 0 {
                    used += 1;
                } else {
                    active += 1;
                }
            }
            (items.len(), active, used, expired, preparing)
        })
        .unwrap_or((0, 0, 0, 0, 0));

    rsx! {
        div {
            // --------------------------------------------------
            // header with bulk actions
            // --------------------------------------------------
            div { class: HEADER_ROW_STY,
                h1 { class: PAGE_TITLE_STY, "Links" }
                div { class: HEADER_ACTIONS_STY,
                    button {
                        class: BTN_XS_STY,
                        disabled: deleting(),
                        onclick: move |_| pending_bulk.set(Some(BulkAction::RemoveUsed)),
                        "Remove Used"
                    }
                    button {
                        class: BTN_XS_STY,
                        disabled: deleting(),
                        onclick: move |_| pending_bulk.set(Some(BulkAction::RemoveExpired)),
                        "Remove Expired"
                    }
                    button {
                        class: BTN_XS_DANGER_STY,
                        disabled: deleting(),
                        onclick: move |_| pending_bulk.set(Some(BulkAction::RemoveAll)),
                        "Remove All"
                    }
                    button {
                        class: BTN_XS_STY,
                        disabled: deleting(),
                        onclick: move |_| links.restart(),
                        if deleting() {
                            span { class: REFRESH_SPIN_STY,
                                SpinnerIcon { class: "w-3 h-3".to_string() }
                                "Refreshing"
                            }
                        } else {
                            "Refresh"
                        }
                    }
                }
            }
            // --------------------------------------------------
            // summary bar
            // --------------------------------------------------
            if total > 0 {
                p { class: SUMMARY_BAR_STY,
                    "{total} links: "
                    span { class: "text-success", "{active} active" }
                    " \u{00B7} "
                    span { class: "text-text-muted", "{used} used" }
                    " \u{00B7} "
                    span { class: "text-danger", "{expired} expired" }
                    if preparing > 0 {
                        " \u{00B7} "
                        span { class: "text-amber-500", "{preparing} preparing" }
                    }
                }
            }
            // --------------------------------------------------
            // table
            // --------------------------------------------------
            div { class: CARD_TABLE_STY,
                match &*links.read_unchecked() {
                    Some(Ok(items)) => {
                        if items.is_empty() {
                            rsx! {
                                div { class: EMPTY_STATE_STY,
                                    "No links yet. Generate one from the Files page."
                                }
                            }
                        } else {
                            rsx! {
                                table { class: "w-full",
                                    thead {
                                        tr { class: TABLE_HEADER_BORDER_STY,
                                            th { class: TABLE_HEADER_STY, "Name" }
                                            th { class: TABLE_HEADER_STY, "Status" }
                                            th { class: TABLE_HEADER_STY, "Downloads" }
                                            th { class: TABLE_HEADER_STY, "Expires" }
                                            th { class: TABLE_HEADER_STY, "Actions" }
                                        }
                                    }
                                    tbody {
                                        for item in items.iter() {
                                            LinkRow {
                                                key: "{item.token}",
                                                item: item.clone(),
                                                links,
                                                on_delete: move |token: String| {
                                                    pending_delete.set(Some(token));
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: ERROR_STATE_STY,
                            "Failed to load links: {e}"
                        }
                    },
                    None => rsx! {
                        div { class: LOADING_STATE_STY,
                            "Loading links..."
                        }
                    },
                }
            }
            // --------------------------------------------------
            // bulk action confirm dialog
            // --------------------------------------------------
            if let Some(action) = &*pending_bulk.read() {
                ConfirmDialog {
                    show: true,
                    title: action.title().to_string(),
                    message: action.message(total),
                    confirm_label: action.label().to_string(),
                    danger: matches!(action, BulkAction::RemoveAll),
                    on_confirm: {
                        let key = action.api_key().to_string();
                        move |_| {
                            let key = key.clone();
                            pending_bulk.set(None);
                            deleting.set(true);
                            spawn(async move {
                                if links::bulk_delete_links(key).await.is_ok() {
                                    links.restart();
                                    toast.push("Links removed", ToastVariant::Success, None);
                                }
                                deleting.set(false);
                            });
                        }
                    },
                    on_cancel: move |_| pending_bulk.set(None),
                }
            }
            // --------------------------------------------------
            // per-link delete confirm dialog
            // --------------------------------------------------
            if let Some(token) = &*pending_delete.read() {
                ConfirmDialog {
                    show: true,
                    title: "Delete Link".to_string(),
                    message: "Delete this link? This cannot be undone.".to_string(),
                    confirm_label: "Delete".to_string(),
                    danger: true,
                    on_confirm: {
                        let t = token.clone();
                        move |_| {
                            let t = t.clone();
                            pending_delete.set(None);
                            spawn(async move {
                                if links::delete_link(t).await.is_ok() {
                                    links.restart();
                                    toast.push("Link deleted", ToastVariant::Success, None);
                                }
                            });
                        }
                    },
                    on_cancel: move |_| pending_delete.set(None),
                }
            }
        }
    }
}
