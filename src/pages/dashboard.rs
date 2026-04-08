//! Dashboard page showing server stats, quick actions, and recent activity
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::core::links;
use crate::pages::styles::PAGE_TITLE_STY;
use crate::requests::TokenListItem;
use crate::routes::AdminRoute;
use crate::svg::{CheckCircleIcon, ClockIcon, DownloadIcon, FolderIcon, PlusIcon};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const STATS_GRID_STY: &str = crate::classes!("grid", "grid-cols-2", "lg:grid-cols-4", "gap-4");

const STAT_CARD_STY: &str = "card";

const STAT_INNER_STY: &str = crate::classes!("flex", "items-center", "gap-3");

const STAT_ICON_ACCENT_STY: &str =
    crate::classes!("p-2", "rounded-lg", "bg-accent/10", "text-accent");

const STAT_ICON_BLUE_STY: &str =
    crate::classes!("p-2", "rounded-lg", "bg-blue-500/10", "text-blue-500");

const STAT_ICON_AMBER_STY: &str =
    crate::classes!("p-2", "rounded-lg", "bg-amber-500/10", "text-amber-500");

const STAT_ICON_RED_STY: &str =
    crate::classes!("p-2", "rounded-lg", "bg-red-500/10", "text-red-500");

const STAT_CARD_LABEL_STY: &str = crate::classes!("text-xs", "text-text-muted", "font-medium");

const STAT_CARD_VALUE_STY: &str = crate::classes!("text-2xl", "font-bold");

const SECTION_GRID_STY: &str =
    crate::classes!("grid", "grid-cols-1", "lg:grid-cols-3", "gap-6", "mt-6");

const SECTION_HEADING_STY: &str = crate::classes!(
    "text-sm",
    "font-semibold",
    "uppercase",
    "tracking-wide",
    "mb-4"
);

const QUICK_ACTION_BTN_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "gap-3",
    "w-full",
    "px-4",
    "py-2.5",
    "text-sm",
    "text-text",
    "border",
    "border-border",
    "rounded-lg",
    "hover:bg-surface-alt",
    "transition-colors"
);

const UPTIME_STY: &str = crate::classes!("text-lg", "font-mono", "text-text-muted");

const ACTIVITY_HEADER_STY: &str = crate::classes!(
    "px-5",
    "py-3",
    "text-sm",
    "font-semibold",
    "border-b",
    "border-border",
    "bg-surface-alt",
    "rounded-t-xl"
);

const ACTIVITY_LIST_STY: &str = crate::classes!(
    "divide-y",
    "divide-border/50",
    "max-h-[400px]",
    "overflow-y-auto"
);

const ACTIVITY_ROW_STY: &str =
    crate::classes!("flex", "items-center", "gap-3", "px-5", "py-3", "text-sm");

const ACTIVITY_NAME_STY: &str = crate::classes!("flex-1", "truncate", "font-medium");

const ACTIVITY_META_STY: &str = crate::classes!("text-xs", "text-text-muted", "tabular-nums");

const ERROR_CARD_STY: &str = crate::classes!("card", "text-danger");

const SKELETON_CARD_STY: &str = crate::classes!("card", "animate-pulse");

const SKELETON_LABEL_STY: &str = crate::classes!("h-4", "w-24", "bg-border", "rounded", "mb-2");

const SKELETON_VALUE_STY: &str = crate::classes!("h-8", "w-16", "bg-border", "rounded");

const TITLE_ROW_STY: &str = crate::classes!("flex", "items-center", "justify-between", "mb-6");

const UPDATED_AGO_STY: &str = crate::classes!("text-xs", "text-text-muted", "tabular-nums");

/// Formats an uptime duration in seconds into a human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

/// Returns a CSS class name for the activity status color
#[inline(always)]
fn activity_status_class(item: &TokenListItem) -> &'static str {
    if item.expired {
        "text-danger"
    } else if item.remaining_downloads == 0 {
        "text-amber-500"
    } else {
        "text-success"
    }
}

#[component]
/// Dashboard page with stat cards, quick actions, and recent activity
pub fn Dashboard() -> Element {
    let mut stats = use_resource(move || async move { links::stats().await });
    let mut activity = use_resource(move || async move { links::list_links().await });

    // --------------------------------------------------
    // signals for local uptime ticking and "updated ago"
    // --------------------------------------------------
    let mut uptime_offset = use_signal(|| 0u64);
    let mut last_server_uptime = use_signal(|| 0u64);
    let mut updated_ago = use_signal(|| 0u64);

    // --------------------------------------------------
    // auto-refresh stats + activity every 30s
    // --------------------------------------------------
    use_future(move || async move {
        loop {
            #[cfg(feature = "web")]
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(30_000)).await;
            stats.restart();
            activity.restart();
        }
    });
    // --------------------------------------------------
    // tick uptime + updated_ago every 1s
    // --------------------------------------------------
    use_future(move || async move {
        loop {
            #[cfg(feature = "web")]
            gloo_timers::future::TimeoutFuture::new(1_000).await;
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(1_000)).await;
            uptime_offset += 1;
            updated_ago += 1;
        }
    });
    // --------------------------------------------------
    // sync server uptime when stats load
    // --------------------------------------------------
    if let Some(Ok(s)) = &*stats.read_unchecked() {
        let server_uptime = s.uptime_seconds;
        if last_server_uptime() != server_uptime {
            last_server_uptime.set(server_uptime);
            uptime_offset.set(0);
            updated_ago.set(0);
        }
    }

    match &*stats.read_unchecked() {
        Some(Ok(s)) => rsx! {
            div {
                div { class: TITLE_ROW_STY,
                    h1 { class: PAGE_TITLE_STY, "Dashboard" }
                    span { class: UPDATED_AGO_STY,
                        "Updated {updated_ago()}s ago"
                    }
                }

                // --------------------------------------------------
                // stat cards
                // --------------------------------------------------
                div { class: STATS_GRID_STY,
                    div { class: STAT_CARD_STY,
                        div { class: STAT_INNER_STY,
                            div { class: STAT_ICON_ACCENT_STY,
                                img { src: crate::LOGO, class: "w-5 h-5" }
                            }
                            div {
                                p { class: STAT_CARD_LABEL_STY, "Active Links" }
                                p { class: STAT_CARD_VALUE_STY, "{s.active_links}" }
                            }
                        }
                    }
                    div { class: STAT_CARD_STY,
                        div { class: STAT_INNER_STY,
                            div { class: STAT_ICON_BLUE_STY,
                                DownloadIcon {}
                            }
                            div {
                                p { class: STAT_CARD_LABEL_STY, "Total Downloads" }
                                p { class: STAT_CARD_VALUE_STY, "{s.total_downloads}" }
                            }
                        }
                    }
                    div { class: STAT_CARD_STY,
                        div { class: STAT_INNER_STY,
                            div { class: STAT_ICON_AMBER_STY,
                                CheckCircleIcon {}
                            }
                            div {
                                p { class: STAT_CARD_LABEL_STY, "Used Links" }
                                p { class: STAT_CARD_VALUE_STY, "{s.used_links}" }
                            }
                        }
                    }
                    div { class: STAT_CARD_STY,
                        div { class: STAT_INNER_STY,
                            div { class: STAT_ICON_RED_STY,
                                ClockIcon {}
                            }
                            div {
                                p { class: STAT_CARD_LABEL_STY, "Expired Links" }
                                p { class: STAT_CARD_VALUE_STY, "{s.expired_links}" }
                            }
                        }
                    }
                }
                // --------------------------------------------------
                // quick actions + uptime and recent activity
                // --------------------------------------------------
                div { class: SECTION_GRID_STY,
                    div {
                        div { class: "card",
                            h2 { class: SECTION_HEADING_STY, "Quick Actions" }
                            div { class: "space-y-2",
                                Link { to: AdminRoute::Browse, class: QUICK_ACTION_BTN_STY,
                                    PlusIcon {}
                                    "New Link"
                                }
                                Link { to: AdminRoute::Browse, class: QUICK_ACTION_BTN_STY,
                                    FolderIcon { class: "w-4 h-4 text-text-muted".to_string() }
                                    "Browse Files"
                                }
                            }
                            div { class: "mt-4 pt-4 border-t border-border",
                                p { class: STAT_CARD_LABEL_STY, "Uptime" }
                                p { class: UPTIME_STY,
                                    "{format_uptime(last_server_uptime() + uptime_offset())}"
                                }
                            }
                        }
                    }
                    div { class: "lg:col-span-2",
                        div { class: "card p-0 overflow-hidden",
                            div { class: ACTIVITY_HEADER_STY,
                                "Recent Activity"
                            }
                            match &*activity.read_unchecked() {
                                Some(Ok(items)) if !items.is_empty() => {
                                    let recent: Vec<&TokenListItem> = items.iter().take(20).collect();
                                    rsx! {
                                        div { class: ACTIVITY_LIST_STY,
                                            for item in recent {
                                                div { key: "{item.token}", class: ACTIVITY_ROW_STY,
                                                    if item.expired {
                                                        ClockIcon { class: "w-4 h-4 text-danger shrink-0".to_string() }
                                                    } else if item.remaining_downloads == 0 {
                                                        CheckCircleIcon { class: "w-4 h-4 text-amber-500 shrink-0".to_string() }
                                                    } else {
                                                        DownloadIcon { class: "w-4 h-4 text-success shrink-0".to_string() }
                                                    }
                                                    span { class: "{ACTIVITY_NAME_STY} {activity_status_class(item)}",
                                                        "{item.name}"
                                                    }
                                                    span { class: ACTIVITY_META_STY,
                                                        "{item.download_count}/{item.max_downloads}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(Ok(_)) => rsx! {
                                    div { class: "px-5 py-8 text-center text-sm text-text-muted",
                                        "No activity yet"
                                    }
                                },
                                Some(Err(_)) => rsx! {
                                    div { class: "px-5 py-8 text-center text-sm text-text-muted",
                                        "Could not load activity"
                                    }
                                },
                                None => rsx! {
                                    div { class: "px-5 py-8 text-center text-sm text-text-muted animate-pulse",
                                        "Loading..."
                                    }
                                },
                            }
                        }
                    }
                }
            }
        },
        Some(Err(e)) => rsx! {
            div {
                h1 { class: PAGE_TITLE_STY, "Dashboard" }
                div { class: ERROR_CARD_STY, "Failed to load stats: {e}" }
            }
        },
        None => rsx! {
            div {
                h1 { class: PAGE_TITLE_STY, "Dashboard" }
                div { class: STATS_GRID_STY,
                    for _ in 0..4u8 {
                        div { class: SKELETON_CARD_STY,
                            div { class: SKELETON_LABEL_STY }
                            div { class: SKELETON_VALUE_STY }
                        }
                    }
                }
            }
        },
    }
}
