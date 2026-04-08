//! Shared UI components
//!
//! Author: aav

// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod confirm;
pub(crate) mod file_row;
pub(crate) mod link_row;
pub(crate) mod menu_icon;
pub(crate) mod modal;
pub(crate) mod toast;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use confirm::ConfirmDialog;
pub use file_row::FileRow;
pub use link_row::LinkRow;
pub use menu_icon::MenuIcon;
pub use modal::Modal;
pub use toast::{Toast, ToastAction, ToastItem, ToastProvider, ToastVariant, Toasts, use_toast};
