//! SVG icon components for the OTD admin panel
//!
//! Author: aav

// --------------------------------------------------
// mods
// --------------------------------------------------
mod icons;
mod mode;
mod spinner;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use icons::{
    CheckCircleIcon, ClockIcon, CoffeeIcon, DownloadIcon, FileIcon, FolderIcon, GitHubIcon,
    GlobeIcon, PlusIcon, XIcon,
};
pub(crate) use mode::{MoonIcon, SunIcon};
pub(crate) use spinner::SpinnerIcon;
