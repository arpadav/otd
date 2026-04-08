//! File browser page for selecting files to share
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    components::{
        FileRow, Modal, ToastVariant,
        file_row::{FILE_ROW_STY, SKELETON_ICON_STY, SKELETON_TEXT_STY},
        use_toast,
    },
    core::{archive::CompressionType, browse, links},
    pages::styles::{EMPTY_STATE_STY, ERROR_STATE_STY, PAGE_TITLE_STY},
    requests::{FileItem, GenerateRequest},
    svg::CheckCircleIcon,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
const BREADCRUMB_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "gap-1",
    "text-sm",
    "text-text-muted",
    "mb-4",
    "flex-wrap"
);
const BREADCRUMB_LINK_STY: &str = crate::classes!("cursor-pointer", "text-accent");
const TOOLBAR_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "justify-between",
    "gap-4",
    "mb-4",
    "flex-wrap"
);
const SEARCH_INPUT_STY: &str = crate::classes!("flex-1", "min-w-0", "max-w-xs");
const SELECTION_ROW_STY: &str = crate::classes!("flex", "items-center", "gap-2");
const SELECTION_COUNT_STY: &str = crate::classes!("text-sm", "text-text-muted");
const BTN_PRIMARY_SM_STY: &str = crate::classes!("btn-primary", "text-sm");
const BTN_SM_STY: &str = crate::classes!("btn", "text-sm");
const CARD_LIST_STY: &str = crate::classes!("card", "overflow-hidden");
const MODAL_BODY_STY: &str = crate::classes!("flex", "flex-col", "gap-3");
const PANEL_SUBTEXT_STY: &str = crate::classes!("text-xs", "text-text-muted");
const FORM_GRID_STY: &str = crate::classes!("grid", "grid-cols-1", "sm:grid-cols-2", "gap-3");
const FORM_ACTIONS_STY: &str = crate::classes!("flex", "items-center", "gap-3");
const RESULT_ERROR_STY: &str = crate::classes!(
    "p-3",
    "text-sm",
    "text-danger",
    "bg-danger-bg",
    "rounded-lg"
);
const SUCCESS_CENTER_STY: &str =
    crate::classes!("flex", "flex-col", "items-center", "gap-3", "py-4");
const SUCCESS_TITLE_STY: &str = crate::classes!("text-lg", "font-semibold", "text-success");
const SUCCESS_URL_STY: &str = crate::classes!(
    "w-full",
    "p-3",
    "text-xs",
    "font-mono",
    "bg-surface-alt",
    "border",
    "border-border",
    "rounded-lg",
    "break-all",
    "select-all"
);
const SUCCESS_ACTIONS_STY: &str = crate::classes!("flex", "items-center", "gap-3", "mt-2");

/// Splits a path into breadcrumb segments with cumulative paths
fn breadcrumb_segments(path: &str) -> Vec<(String, String)> {
    if path.is_empty() {
        return vec![];
    }
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut segments = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let full_path = parts[..=i].join("/");
        segments.push((part.to_string(), full_path));
    }
    segments
}

#[component]
/// File browser page with directory listing, selection, and link generation
pub fn Browse() -> Element {
    let mut current_path = use_signal(String::new);
    let mut selected = use_signal(Vec::<String>::new);
    let mut search = use_signal(String::new);
    let mut show_generate = use_signal(|| false);
    let mut gen_name = use_signal(String::new);
    let mut gen_max_dl = use_signal(|| "1".to_string());
    let mut gen_expires = use_signal(String::new);
    let mut gen_format = use_signal(|| "zip".to_string());
    let mut gen_result = use_signal(|| None::<Result<String, String>>);
    let mut generating = use_signal(|| false);
    let toast = use_toast();

    let mut listing = use_resource(move || {
        let p = current_path();
        async move { browse::browse(p).await }
    });

    let handle_generate = move |_| {
        let sel = selected();
        if sel.is_empty() {
            return;
        }
        generating.set(true);
        gen_result.set(None);

        let name = {
            let v = gen_name();
            if v.is_empty() { None } else { Some(v) }
        };
        let max_downloads = gen_max_dl().parse::<u32>().ok();
        let expires_in_seconds = {
            let v = gen_expires();
            if v.is_empty() {
                None
            } else {
                v.parse::<u64>().ok()
            }
        };
        let format = match gen_format().as_str() {
            "tar" => CompressionType::Tar,
            "targz" => CompressionType::TarGz,
            _ => CompressionType::Zip,
        };

        spawn(async move {
            let req = GenerateRequest {
                paths: sel,
                name,
                max_downloads,
                expires_in_seconds,
                format,
            };
            match links::generate_link(req).await {
                Ok(resp) => {
                    gen_result.set(Some(Ok(resp.download_url)));
                    selected.write().clear();
                }
                Err(e) => {
                    gen_result.set(Some(Err(e.to_string())));
                }
            }
            generating.set(false);
        });
    };

    let segments = breadcrumb_segments(&current_path());

    rsx! {
        div {
            h1 { class: "{PAGE_TITLE_STY} mb-4", "Files" }
            // --------------------------------------------------
            // breadcrumbs
            // --------------------------------------------------
            div { class: BREADCRUMB_STY,
                span {
                    class: BREADCRUMB_LINK_STY,
                    onclick: move |_| {
                        current_path.set(String::new());
                        selected.write().clear();
                        search.set(String::new());
                        show_generate.set(false);
                        listing.restart();
                    },
                    "Root"
                }
                for (name, path) in segments {
                    span { class: "text-text-muted", "/" }
                    span {
                        class: BREADCRUMB_LINK_STY,
                        onclick: move |_| {
                            current_path.set(path.clone());
                            selected.write().clear();
                            search.set(String::new());
                            show_generate.set(false);
                            listing.restart();
                        },
                        "{name}"
                    }
                }
            }
            // --------------------------------------------------
            // toolbar: search + selection info
            // --------------------------------------------------
            div { class: TOOLBAR_STY,
                input {
                    r#type: "search",
                    placeholder: "Filter files...",
                    class: SEARCH_INPUT_STY,
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
                if !selected().is_empty() {
                    div { class: SELECTION_ROW_STY,
                        span { class: SELECTION_COUNT_STY,
                            "{selected().len()} selected"
                        }
                        button {
                            class: BTN_PRIMARY_SM_STY,
                            onclick: move |_| show_generate.set(!show_generate()),
                            "Generate Link"
                        }
                        button {
                            class: BTN_SM_STY,
                            onclick: move |_| selected.write().clear(),
                            "Clear"
                        }
                    }
                }
            }
            // --------------------------------------------------
            // file listing
            // --------------------------------------------------
            div { class: CARD_LIST_STY,
                match &*listing.read_unchecked() {
                    Some(Ok(items)) => {
                        let search_val = search().to_lowercase();
                        let filtered: Vec<&FileItem> = items
                            .iter()
                            .filter(|item| {
                                search_val.is_empty() || item.name.to_lowercase().contains(&search_val)
                            })
                            .collect();

                        if filtered.is_empty() {
                            rsx! {
                                div { class: EMPTY_STATE_STY,
                                    if items.is_empty() {
                                        "This directory is empty"
                                    } else {
                                        "No files match your filter"
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                for item in filtered {
                                    FileRow {
                                        key: "{item.path}",
                                        is_selected: selected().contains(&item.path),
                                        item: item.clone(),
                                        current_path,
                                        selected,
                                        search,
                                        show_generate,
                                        listing,
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: ERROR_STATE_STY,
                            "Error: {e}"
                        }
                    },
                    None => rsx! {
                        for i in 0..6u8 {
                            div { key: "{i}", class: "{FILE_ROW_STY} animate-pulse",
                                div { class: SKELETON_ICON_STY }
                                div { class: SKELETON_ICON_STY }
                                div { class: SKELETON_TEXT_STY }
                            }
                        }
                    },
                }
            }
            // --------------------------------------------------
            // generate link modal
            // --------------------------------------------------
            Modal {
                show: show_generate() && (!selected().is_empty() || gen_result.read().is_some()),
                on_close: move |_| {
                    show_generate.set(false);
                    gen_result.set(None);
                },
                title: "Generate Download Link".to_string(),

                div { class: MODAL_BODY_STY,
                    // --------------------------------------------------
                    // result state: success, error, or form
                    // --------------------------------------------------
                    match &*gen_result.read() {
                        Some(Ok(url)) => rsx! {
                            div { class: SUCCESS_CENTER_STY,
                                CheckCircleIcon { class: "w-10 h-10 text-success".to_string() }
                                h3 { class: SUCCESS_TITLE_STY, "Link Created!" }
                                div { class: SUCCESS_URL_STY, "{url}" }
                                div { class: SUCCESS_ACTIONS_STY,
                                    button {
                                        class: "btn-primary",
                                        onclick: {
                                            let url = url.clone();
                                            move |_| {
                                                let url = url.clone();
                                                document::eval(&format!(
                                                    "navigator.clipboard.writeText('{url}')"
                                                ));
                                                toast.push(
                                                    "URL copied to clipboard",
                                                    ToastVariant::Success,
                                                    None,
                                                );
                                            }
                                        },
                                        "Copy URL"
                                    }
                                    button {
                                        class: "btn",
                                        onclick: move |_| {
                                            gen_result.set(None);
                                            gen_name.set(String::new());
                                        },
                                        "Create Another"
                                    }
                                    button {
                                        class: "btn",
                                        onclick: move |_| {
                                            show_generate.set(false);
                                            gen_result.set(None);
                                        },
                                        "Close"
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: RESULT_ERROR_STY, "Error: {e}" }
                            div { class: FORM_ACTIONS_STY,
                                button {
                                    class: "btn",
                                    onclick: move |_| gen_result.set(None),
                                    "Try Again"
                                }
                            }
                        },
                        None => rsx! {
                            p { class: PANEL_SUBTEXT_STY,
                                "Creating link for {selected().len()} item(s)"
                            }

                            div { class: FORM_GRID_STY,
                                div {
                                    label { class: "label", "Custom Name (optional)" }
                                    input {
                                        r#type: "text",
                                        class: "w-full",
                                        placeholder: "auto-generated",
                                        value: "{gen_name}",
                                        oninput: move |e| gen_name.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "label", "Max Downloads" }
                                    input {
                                        r#type: "number",
                                        class: "w-full",
                                        value: "{gen_max_dl}",
                                        min: "1",
                                        oninput: move |e| gen_max_dl.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "label", "Expires In (seconds)" }
                                    input {
                                        r#type: "number",
                                        class: "w-full",
                                        placeholder: "never",
                                        value: "{gen_expires}",
                                        oninput: move |e| gen_expires.set(e.value()),
                                    }
                                }
                                div {
                                    label { class: "label", "Format" }
                                    select {
                                        class: "w-full",
                                        value: "{gen_format}",
                                        onchange: move |e| gen_format.set(e.value()),
                                        option { value: "zip", "ZIP" }
                                        option { value: "targz", "TAR.GZ" }
                                        option { value: "tar", "TAR" }
                                    }
                                }
                            }

                            div { class: FORM_ACTIONS_STY,
                                button {
                                    class: "btn-primary",
                                    disabled: generating(),
                                    onclick: handle_generate,
                                    if generating() { "Generating..." } else { "Create Link" }
                                }
                                button {
                                    class: "btn",
                                    onclick: move |_| show_generate.set(false),
                                    "Cancel"
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
