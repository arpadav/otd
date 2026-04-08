//! Shared style constants used across multiple pages
//!
//! Author: aav

// --------------------------------------------------
// style
// --------------------------------------------------
/// Page title: `text-2xl font-bold`
pub const PAGE_TITLE_STY: &str = crate::classes!("text-2xl", "font-bold");

/// Base for centered state messages (empty, error, loading)
pub const CENTER_STATE_BASE: &str = crate::classes!("px-4", "py-8", "text-center", "text-sm");

/// Empty state: muted text
pub const EMPTY_STATE_STY: &str = crate::classes!(CENTER_STATE_BASE, "text-text-muted");

/// Error state: danger text
pub const ERROR_STATE_STY: &str = crate::classes!(CENTER_STATE_BASE, "text-danger");

/// Loading state: muted text with pulse animation
pub const LOADING_STATE_STY: &str =
    crate::classes!(CENTER_STATE_BASE, "text-text-muted", "animate-pulse");

/// Base for status badge pills
pub const STATUS_BADGE_BASE: &str = crate::classes!(
    "inline-flex",
    "items-center",
    "px-2",
    "py-0.5",
    "rounded-full",
    "text-xs",
    "font-medium"
);
