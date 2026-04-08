//! File browser row component
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::requests::FileItem;
use crate::svg::{FileIcon, FolderIcon};

// --------------------------------------------------
// external
// --------------------------------------------------
use dioxus::prelude::*;

// --------------------------------------------------
// style
// --------------------------------------------------
pub const FILE_ROW_STY: &str = crate::classes!(
    "flex",
    "items-center",
    "gap-3",
    "px-4",
    "py-2.5",
    "border-b",
    "border-border",
    "transition-colors",
    "duration-150"
);
const FILE_NAME_DIR_STY: &str =
    crate::classes!("text-sm", "font-medium", "text-accent", "cursor-pointer");
const FILE_NAME_STY: &str = crate::classes!("text-sm", "font-medium");
const FILE_SIZE_STY: &str =
    crate::classes!("text-xs", "text-text-muted", "ml-auto", "tabular-nums");
pub const SKELETON_ICON_STY: &str = crate::classes!("w-4", "h-4", "bg-border", "rounded");
pub const SKELETON_TEXT_STY: &str = crate::classes!("h-4", "w-32", "bg-border", "rounded");

// --------------------------------------------------
// helpers
// --------------------------------------------------
/// Formats a byte count into a human-readable size string
#[inline]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    match bytes {
        b if b >= GB => format!("{:.1} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
        b => format!("{b} B"),
    }
}

// --------------------------------------------------
// component
// --------------------------------------------------
#[component]
/// A single row in the file browser listing
pub fn FileRow(
    item: FileItem,
    is_selected: bool,
    mut current_path: Signal<String>,
    mut selected: Signal<Vec<String>>,
    mut search: Signal<String>,
    mut show_generate: Signal<bool>,
    mut listing: Resource<Result<Vec<FileItem>>>,
) -> Element {
    let is_parent = item.name == "..";
    let is_dir = item.is_dir;
    let path = item.path.clone();
    let name = item.name.clone();
    let size = item.size;

    rsx! {
        div { class: FILE_ROW_STY,
            // Checkbox (not for parent dir)
            if !is_parent {
                input {
                    r#type: "checkbox",
                    checked: is_selected,
                    class: "cursor-pointer",
                    onchange: {
                        let p = path.clone();
                        move |_| {
                            let mut sel = selected.write();
                            if let Some(pos) = sel.iter().position(|s| *s == p) {
                                sel.remove(pos);
                            } else {
                                sel.push(p.clone());
                            }
                        }
                    },
                }
            } else {
                div { class: "w-4" }
            }

            // Icon
            if is_dir {
                FolderIcon {}
            } else {
                FileIcon {}
            }

            // Name
            if is_dir {
                span {
                    class: FILE_NAME_DIR_STY,
                    onclick: {
                        let p = path.clone();
                        move |_| {
                            current_path.set(p.clone());
                            selected.write().clear();
                            search.set(String::new());
                            show_generate.set(false);
                            listing.restart();
                        }
                    },
                    "{name}"
                }
            } else {
                span { class: FILE_NAME_STY, "{name}" }
            }

            // Size
            if let Some(bytes) = size {
                span { class: FILE_SIZE_STY, "{format_size(bytes)}" }
            }
        }
    }
}
