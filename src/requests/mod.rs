//! Request and response DTO types shared across server functions and client pages
//!
//! All public types here are available on both WASM and server compilation
//! targets. Server-only types are gated with `#[cfg(feature = "server")]`
//!
//! # Module layout
//!
//! | Module | Type(s) | Server-only |
//! |---|---|---|
//! | [`browse`] | [`FileItem`] | no |
//! | [`delete`] | [`BulkDeleteResponse`] | no |
//! | [`generate`] | [`GenerateRequest`], [`GenerateResponse`] | no |
//! | [`links`] | [`TokenListItem`] | no |
//! | [`persisted`] | `PersistedDownloadItem` | yes |
//! | [`stats`] | [`StatsResponse`] | no |
//!
//! Author: aav

// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod browse;
pub mod delete;
pub mod generate;
pub mod links;
#[cfg(feature = "server")]
pub mod persisted;
pub mod stats;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use browse::FileItem;
pub use delete::BulkDeleteResponse;
pub use generate::{GenerateRequest, GenerateResponse};
pub use links::TokenListItem;
pub use stats::StatsResponse;

#[cfg(feature = "server")]
pub(crate) use persisted::PersistedDownloadItem;
