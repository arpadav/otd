//! Link table row and status badge components
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::components::toast::{ToastVariant, use_toast};
use crate::core::links;
use crate::pages::styles::STATUS_BADGE_BASE;
use crate::requests::TokenListItem;
use crate::svg::SpinnerIcon;

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const STATUS_ACTIVE_STY: &str = crate::classes!(STATUS_BADGE_BASE, "bg-success-bg", "text-success");
const STATUS_USED_STY: &str =
    crate::classes!(STATUS_BADGE_BASE, "bg-warning-bg", "text-text-muted");
const STATUS_EXPIRED_STY: &str = crate::classes!(STATUS_BADGE_BASE, "bg-danger-bg", "text-danger");
const STATUS_PREPARING_STY: &str = crate::classes!(
    STATUS_BADGE_BASE,
    "gap-1",
    "bg-warning-bg",
    "text-amber-700",
    "dark:text-amber-300"
);
const STATUS_FAILED_STY: &str = crate::classes!(STATUS_BADGE_BASE, "bg-danger-bg", "text-danger");
const STATUS_SOURCE_MISSING_STY: &str = crate::classes!(
    STATUS_BADGE_BASE,
    "bg-gray-100",
    "dark:bg-gray-800",
    "text-gray-500"
);

const TABLE_CELL_STY: &str = crate::classes!("px-4", "py-3", "text-sm");
const TABLE_CELL_NUM_STY: &str = crate::classes!("px-4", "py-3", "text-sm", "tabular-nums");
const NAME_STY: &str = "font-medium";
const ARCHIVE_TAG_STY: &str = crate::classes!("ml-1", "text-xs", "text-text-muted");
const TOKEN_PREVIEW_STY: &str = crate::classes!(
    "text-xs",
    "text-text-muted",
    "mt-0.5",
    "truncate",
    "max-w-xs"
);
const ACTIONS_STY: &str = crate::classes!("flex", "items-center", "gap-2");
const BTN_XS_STY: &str = "btn-xs";
const BTN_XS_DANGER_STY: &str = "btn-xs-danger";
const REVIVE_BTN_STY: &str = crate::classes!(
    "px-2",
    "py-0.5",
    "rounded-full",
    "text-xs",
    "font-medium",
    "bg-warning-bg",
    "text-amber-700",
    "dark:text-amber-300",
    "hover:bg-amber-200",
    "dark:hover:bg-amber-800",
    "transition-colors",
    "cursor-pointer"
);

/// Formats an expiry duration into a human-readable string
pub fn format_expiry(secs: Option<u64>, expired: bool) -> String {
    if expired {
        return "Expired".into();
    }
    match secs {
        Some(s) if s >= 86400 => format!("{}d {}h", s / 86400, (s % 86400) / 3600),
        Some(s) if s >= 3600 => format!("{}h {}m", s / 3600, (s % 3600) / 60),
        Some(s) if s >= 60 => format!("{}m {}s", s / 60, s % 60),
        Some(s) => format!("{s}s"),
        None => "Never".into(),
    }
}

/// Renders the status badge for a link based on priority:
/// Source Missing > Expired > Used > Failed > Preparing > Active
#[component]
fn StatusBadge(item: TokenListItem) -> Element {
    if !item.source_exists {
        return rsx! {
            span { class: STATUS_SOURCE_MISSING_STY, "Source Missing" }
        };
    }
    if item.expired {
        return rsx! {
            span { class: STATUS_EXPIRED_STY, "Expired" }
        };
    }
    if item.remaining_downloads == 0 {
        return rsx! {
            span { class: STATUS_USED_STY, "Used" }
        };
    }
    if item.archive_status == "failed" {
        return rsx! {
            span { class: STATUS_FAILED_STY, "Failed" }
        };
    }
    if item.archive_status == "preparing" {
        return rsx! {
            span { class: STATUS_PREPARING_STY,
                SpinnerIcon {}
                "Preparing"
            }
        };
    }
    rsx! {
        span { class: STATUS_ACTIVE_STY, "Active" }
    }
}

#[component]
/// A single row in the links management table
pub fn LinkRow(
    item: TokenListItem,
    mut links: Resource<Result<Vec<TokenListItem>>>,
    on_delete: EventHandler<String>,
) -> Element {
    let toast = use_toast();
    let url = item.download_url.clone();
    let token = item.token.clone();
    let source_exists = item.source_exists;
    let show_revive = item.archive_status == "failed" && item.source_exists;

    let copy_url = {
        let url = url.clone();
        move |_| {
            let url = url.clone();
            document::eval(&format!("navigator.clipboard.writeText('{url}')"));
            toast.push("URL copied to clipboard", ToastVariant::Success, None);
        }
    };

    let request_delete = {
        let t = token.clone();
        move |_| {
            on_delete.call(t.clone());
        }
    };

    let revive = {
        let t = token.clone();
        move |_| {
            let t = t.clone();
            spawn(async move {
                if links::revive_link(t).await.is_ok() {
                    links.restart();
                }
            });
        }
    };

    let row_class = crate::classes_rt!(
        "border-b", "border-border";
        (!source_exists) => "opacity-60",
    );

    rsx! {
        tr { class: "{row_class}",
            // --------------------------------------------------
            // name cell
            // --------------------------------------------------
            td { class: TABLE_CELL_STY,
                div {
                    span { class: NAME_STY, "{item.name}" }
                    if item.is_multi_file {
                        span { class: ARCHIVE_TAG_STY, "(archive)" }
                    }
                }
                div { class: TOKEN_PREVIEW_STY,
                    "{item.token}"
                }
            }
            // --------------------------------------------------
            // status cell
            // --------------------------------------------------
            td { class: TABLE_CELL_STY,
                StatusBadge { item: item.clone() }
            }
            // --------------------------------------------------
            // downloads cell
            // --------------------------------------------------
            td { class: TABLE_CELL_NUM_STY,
                "{item.download_count} / {item.max_downloads}"
            }
            // --------------------------------------------------
            // expiry cell
            // --------------------------------------------------
            td { class: TABLE_CELL_NUM_STY,
                "{format_expiry(item.expires_in_seconds, item.expired)}"
            }
            // --------------------------------------------------
            // actions cell
            // --------------------------------------------------
            td { class: TABLE_CELL_STY,
                div { class: ACTIONS_STY,
                    button {
                        class: BTN_XS_STY,
                        onclick: copy_url,
                        title: "Copy download URL",
                        "Copy"
                    }
                    if show_revive {
                        button {
                            class: REVIVE_BTN_STY,
                            onclick: revive,
                            title: "Re-create archive from source files",
                            "Revive"
                        }
                    }
                    button {
                        class: BTN_XS_DANGER_STY,
                        onclick: request_delete,
                        title: "Delete link",
                        "Delete"
                    }
                }
            }
        }
    }
}
