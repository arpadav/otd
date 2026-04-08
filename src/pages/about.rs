//! About page with project info, links, and support
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::pages::styles::PAGE_TITLE_STY;
use crate::svg::{CoffeeIcon, GitHubIcon, GlobeIcon};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const SUBTITLE_STY: &str = crate::classes!("mt-1", "text-sm", "text-text-muted");
const CARD_STY: &str = crate::classes!("card", "space-y-6");
const BODY_TEXT_STY: &str =
    crate::classes!("space-y-4", "text-sm", "leading-relaxed", "text-text/80");
const SECTION_HEADING_STY: &str = crate::classes!(
    "text-sm",
    "font-semibold",
    "text-text",
    "uppercase",
    "tracking-wide"
);
const LINKS_LIST_STY: &str = crate::classes!("space-y-2", "text-sm");
const LINK_ROW_STY: &str = crate::classes!(
    "inline-flex",
    "items-center",
    "gap-2",
    "text-accent",
    "hover:text-accent-hover",
    "transition-colors"
);
const SUPPORT_TEXT_STY: &str = crate::classes!("text-sm", "text-text-muted");
const SUPPORT_ROW_STY: &str = crate::classes!("flex", "flex-col", "sm:flex-row", "gap-3");
const COFFEE_BTN_STY: &str = crate::classes!(
    "inline-flex",
    "items-center",
    "justify-center",
    "gap-2",
    "px-4",
    "py-2",
    "text-sm",
    "font-semibold",
    "text-yellow-900",
    "bg-yellow-400",
    "rounded-lg",
    "hover:bg-yellow-300",
    "transition-colors"
);
const BTC_BADGE_STY: &str = crate::classes!(
    "inline-flex",
    "items-center",
    "gap-2",
    "px-4",
    "py-2",
    "text-sm",
    "text-text-muted",
    "bg-surface-alt",
    "border",
    "border-border",
    "rounded-lg"
);

#[component]
/// About page with project description, external links, and support options
pub fn About() -> Element {
    rsx! {
        div {
            div { class: "mb-6",
                h1 { class: PAGE_TITLE_STY, "About OTD" }
                p { class: SUBTITLE_STY, "One-Time Downloads" }
            }
            div { class: CARD_STY,
                div { class: BODY_TEXT_STY,
                    p {
                        "OTD is a lightweight, self-hosted tool for creating "
                        strong { class: "text-text", "secure, one-time download links" }
                        ". Share files with anyone \u{2014} once the link is used, it\u{2019}s gone. No accounts, no cloud storage, no tracking."
                    }
                    p {
                        "Built with "
                        strong { class: "text-text", "Rust" }
                        " for performance and reliability. No JavaScript runtime, no container orchestration, no dependencies you don\u{2019}t need. Just a single binary that does one thing well."
                    }
                    p {
                        "OTD is "
                        strong { class: "text-text", "free and open source" }
                        ". Use it, modify it, host it yourself."
                    }
                }
                hr { class: "border-border" }
                // Links
                div { class: "space-y-3",
                    h2 { class: SECTION_HEADING_STY, "Links" }
                    ul { class: LINKS_LIST_STY,
                        li {
                            a {
                                href: "https://github.com/arpadav/otd",
                                target: "_blank",
                                rel: "noopener",
                                class: LINK_ROW_STY,
                                GitHubIcon {}
                                "GitHub"
                            }
                        }
                        li {
                            a {
                                href: "https://arpadvoros.com",
                                target: "_blank",
                                rel: "noopener",
                                class: LINK_ROW_STY,
                                GlobeIcon {}
                                "arpadvoros.com"
                            }
                        }
                    }
                }
                hr { class: "border-border" }
                // Support
                div { class: "space-y-3",
                    h2 { class: SECTION_HEADING_STY, "Support" }
                    p { class: SUPPORT_TEXT_STY,
                        "If OTD is useful to you, consider supporting the project."
                    }
                    div { class: SUPPORT_ROW_STY,
                        a {
                            href: "https://buymeacoffee.com/arpadav",
                            target: "_blank",
                            rel: "noopener",
                            class: COFFEE_BTN_STY,
                            CoffeeIcon {}
                            "Buy Me a Coffee"
                        }
                        div { class: BTC_BADGE_STY,
                            span { "\u{20BF}" }
                            code { class: "font-mono text-xs", "bc1q..." }
                        }
                    }
                }
            }
        }
    }
}
