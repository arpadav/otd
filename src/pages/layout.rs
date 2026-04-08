//! Main layout component wrapping all pages
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::components::{MenuIcon, ToastItem, ToastProvider};
use crate::routes::AdminRoute;
use crate::svg::{CoffeeIcon, MoonIcon, SunIcon};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const ROOT_STY: &str = crate::classes!("flex", "flex-col", "min-h-screen", "bg-surface-alt");

const NAV_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "justify-between",
    "px-6",
    "py-3",
    "border-b",
    "border-border",
    "bg-surface",
    "sticky",
    "top-0",
    "z-40"
);

const LOGO_GROUP_STY: &str = crate::classes!("flex", "items-center", "gap-2");

const LOGO_IMG_STY: &str = crate::classes!("w-6", "h-6");

const LOGO_TEXT_STY: &str = crate::classes!("font-bold", "text-lg");

const NAV_LINKS_STY: &str = crate::classes!("hidden", "sm:flex", "items-center", "gap-1");

const NAV_LINK_STY: &str = crate::classes!(
    "px-3",
    "py-1.5",
    "text-sm",
    "rounded-md",
    "text-text-muted",
    "transition-colors",
    "duration-200"
);

const NAV_LINK_ACTIVE_STY: &str = crate::classes!(
    "px-3",
    "py-1.5",
    "text-sm",
    "rounded-md",
    "text-accent",
    "bg-accent/10",
    "font-medium",
    "transition-colors",
    "duration-200"
);

const NAV_RIGHT_STY: &str = crate::classes!("flex", "items-center", "gap-1");

const TOGGLE_BTN_STY: &str = crate::classes!(
    "p-2",
    "rounded-md",
    "text-text-muted",
    "transition-colors",
    "duration-200",
    "cursor-pointer"
);

const MOBILE_TOGGLE_STY: &str = crate::classes!(
    "p-2",
    "rounded-md",
    "text-text-muted",
    "transition-colors",
    "duration-200",
    "cursor-pointer",
    "sm:hidden"
);

const MOBILE_MENU_STY: &str = crate::classes!(
    "sm:hidden",
    "border-b",
    "border-border",
    "bg-surface",
    "px-4",
    "py-2",
    "flex",
    "flex-col",
    "gap-1"
);

const MOBILE_LINK_STY: &str = crate::classes!(
    "block",
    "w-full",
    "text-left",
    "px-3",
    "py-2",
    "text-sm",
    "rounded-md",
    "text-text-muted",
    "transition-colors",
    "duration-200"
);

const MOBILE_LINK_ACTIVE_STY: &str = crate::classes!(
    "block",
    "w-full",
    "text-left",
    "px-3",
    "py-2",
    "text-sm",
    "rounded-md",
    "text-accent",
    "bg-accent/10",
    "font-medium",
    "transition-colors",
    "duration-200"
);

const MAIN_STY: &str = crate::classes!("flex-1", "p-6", "max-w-6xl", "mx-auto", "w-full");

// --------------------------------------------------
// footer
// --------------------------------------------------
const FOOTER_STY: &str = crate::classes!("border-t", "border-border", "bg-brand", "text-slate-400");

const FOOTER_INNER_STY: &str = crate::classes!(
    "flex",
    "flex-col",
    "sm:flex-row",
    "items-center",
    "justify-center",
    "gap-3",
    "px-4",
    "sm:px-6",
    "lg:px-8",
    "py-4",
    "max-w-6xl",
    "mx-auto",
    "text-xs"
);

const FOOTER_LINK_STY: &str = crate::classes!("hover:text-white", "transition-colors");

const FOOTER_SEP_STY: &str = crate::classes!("hidden", "sm:inline", "text-slate-600");

const FOOTER_COFFEE_STY: &str = crate::classes!(
    "inline-flex",
    "items-center",
    "gap-1",
    "hover:text-yellow-300",
    "transition-colors"
);

const FOOTER_BTC_STY: &str = crate::classes!(
    "font-mono",
    "text-slate-500",
    "hover:text-white",
    "transition-colors",
    "cursor-pointer"
);

/// Navigation item with a label and route
struct NavItem {
    /// Display text for the navigation link
    label: &'static str,
    /// Target route
    route: AdminRoute,
}

/// Ordered list of navigation items for both desktop and mobile menus
const NAV_ITEMS: &[NavItem] = &[
    NavItem {
        label: "Dashboard",
        route: AdminRoute::Dashboard,
    },
    NavItem {
        label: "Files",
        route: AdminRoute::Browse,
    },
    NavItem {
        label: "Links",
        route: AdminRoute::Links,
    },
    NavItem {
        label: "About",
        route: AdminRoute::About,
    },
];

#[component]
/// Main layout wrapping all pages with navbar, footer, and toast provider
pub fn Layout() -> Element {
    let mut dark_mode = use_signal(|| false);
    let mut mobile_open = use_signal(|| false);
    let current_route = use_route::<AdminRoute>();

    // --------------------------------------------------
    // toggle dark class on document element via eval
    // --------------------------------------------------
    let toggle_dark = move |_| {
        let new_val = !dark_mode();
        dark_mode.set(new_val);
        let js = if new_val {
            "document.documentElement.classList.add('dark'); localStorage.setItem('otd-dark', '1');"
        } else {
            "document.documentElement.classList.remove('dark'); localStorage.setItem('otd-dark', '0');"
        };
        document::eval(js);
    };

    // --------------------------------------------------
    // initialize dark mode from localStorage on mount
    // --------------------------------------------------
    use_hook(|| {
        document::eval(
            r#"
            if (localStorage.getItem('otd-dark') === '1' || (!localStorage.getItem('otd-dark') && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
                document.documentElement.classList.add('dark');
                window.__otd_dark = true;
            }
            "#,
        );
    });

    let is_active = |route: &AdminRoute| -> bool {
        std::mem::discriminant(&current_route) == std::mem::discriminant(route)
    };

    // --------------------------------------------------
    // provide toast context to all child pages
    // --------------------------------------------------
    use_context_provider(|| Signal::new(Vec::<ToastItem>::new()));

    rsx! {
        div { class: ROOT_STY,
            // --------------------------------------------------
            // navbar
            // --------------------------------------------------
            nav { class: NAV_STY,
                Link { to: AdminRoute::Dashboard,
                    div { class: LOGO_GROUP_STY,
                        img { src: crate::LOGO, class: LOGO_IMG_STY }
                        span { class: LOGO_TEXT_STY, "OTD" }
                    }
                }
                div { class: NAV_LINKS_STY,
                    for item in NAV_ITEMS.iter() {
                        Link {
                            to: item.route.clone(),
                            class: if is_active(&item.route) { NAV_LINK_ACTIVE_STY } else { NAV_LINK_STY },
                            "{item.label}"
                        }
                    }
                }
                div { class: NAV_RIGHT_STY,
                    button {
                        class: TOGGLE_BTN_STY,
                        onclick: toggle_dark,
                        title: "Toggle dark mode",
                        if dark_mode() {
                            SunIcon {}
                        } else {
                            MoonIcon {}
                        }
                    }
                    button {
                        class: MOBILE_TOGGLE_STY,
                        onclick: move |_| mobile_open.set(!mobile_open()),
                        MenuIcon {}
                    }
                }
            }
            // --------------------------------------------------
            // mobile menu
            // --------------------------------------------------
            if mobile_open() {
                div { class: MOBILE_MENU_STY,
                    for item in NAV_ITEMS.iter() {
                        Link {
                            to: item.route.clone(),
                            class: if is_active(&item.route) { MOBILE_LINK_ACTIVE_STY } else { MOBILE_LINK_STY },
                            onclick: move |_| mobile_open.set(false),
                            "{item.label}"
                        }
                    }
                }
            }
            // --------------------------------------------------
            // main content
            // --------------------------------------------------
            main { class: MAIN_STY,
                Outlet::<AdminRoute> {}
            }
            // --------------------------------------------------
            // toast provider: fixed position, renders above all
            // --------------------------------------------------
            ToastProvider {}
            // --------------------------------------------------
            // footer
            // --------------------------------------------------
            footer { class: FOOTER_STY,
                div { class: FOOTER_INNER_STY,
                    Link { to: AdminRoute::About, class: FOOTER_LINK_STY, "About OTD" }
                    span { class: FOOTER_SEP_STY, "\u{00B7}" }
                    a {
                        href: "https://arpadvoros.com",
                        target: "_blank",
                        rel: "noopener",
                        class: FOOTER_LINK_STY,
                        "arpadvoros.com"
                    }
                    span { class: FOOTER_SEP_STY, "\u{00B7}" }
                    a {
                        href: "https://buymeacoffee.com/arpadav",
                        target: "_blank",
                        rel: "noopener",
                        class: FOOTER_COFFEE_STY,
                        CoffeeIcon { class: "w-3.5 h-3.5".to_string() }
                        "Buy Me a Coffee"
                    }
                    span { class: FOOTER_SEP_STY, "\u{00B7}" }
                    button {
                        class: FOOTER_BTC_STY,
                        title: "Copy BTC address",
                        onclick: move |_| {
                            document::eval("navigator.clipboard.writeText('bc1q...')");
                        },
                        "\u{20BF} bc1q..."
                    }
                }
            }
        }
    }
}
