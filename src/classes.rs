#![allow(unused_imports, dead_code)]
//! # Tailwind Classes
//!
//! Use Tailwind-Intellisense with macros, to not have Tailwind syntax clog up your editor
//!
//! You can use [`tw::classes!`](classes!) to combine Tailwind classes into a single space-separated string with proper syntax highlighting, autocompletion, and spacing:
//!
//! ```rust
//! let button_classes = tw::classes!(
//!     "bg-blue-500",
//!     "inline-flex",
//!     "items-center",
//!     "justify-center",
//!     "text-white",
//!     "px-4",
//!     "py-2",
//!     "rounded-md",
//! );
//! assert_eq!(button_classes, "bg-blue-500 inline-flex items-center justify-center text-white px-4 py-2 rounded-md");
//! ```
//!
//! The resulting string will be highlighted with the appropriate Tailwind class syntax, and the spacing will be preserved
//!
//! # Vscode Settings
//!
//! Add the following to your `.vscode/settings.json`:
//!
//! ```json
//! "tailwindCSS.experimental.classRegex": [
//!   // 1. const NAME_(TW|STY): &str = "...";
//!   [
//!     "\\bconst\\s+[A-Z_][A-Z0-9_]*(?:_TW|_STY)\\s*:\\s*&?str\\s*=\\s*([^;]*);",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 2. const|let|static NAME = classes! | class_if | classes_rt! "..." ;
//!   // this is optimized since ends with semicolon, fast match
//!   [
//!     "\\b(?:const|let|static)\\s+\\w+\\s*=\\s*(?:\\w+::)*(?:classes!|class_if|classes_rt!)[^;]*;",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 3a. classes! ( ... )
//!   [
//!     "(?:\\b\\w+::)?classes!\\s*\\(((?:[^()]|\\([^()]*\\))*?)\\)",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 3b. classes! { ... }
//!   [
//!     "(?:\\b\\w+::)?classes!\\s*\\{((?:[^{}]|\\{[^{}]*\\})*?)\\}",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 3c. classes! [ ... ]
//!   [
//!     "(?:\\b\\w+::)?classes!\\s*\\[((?:[^\\[\\]]|\\[[^\\[\\]]*\\])*?)\\]",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 4. class_if ( ... )
//!   [
//!     "(?:\\b\\w+::)?class_if\\s*\\(((?:[^()]|\\([^()]*\\))*?)\\)",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 5a. classes_rt! ( ... )
//!   [
//!     "(?:\\b\\w+::)?classes_rt!\\s*\\(((?:[^()]|\\([^()]*\\))*?)\\)",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 5b. classes_rt! { ... }
//!   [
//!     "(?:\\b\\w+::)?classes_rt!\\s*\\{((?:[^{}]|\\{[^{}]*\\})*?)\\}",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ],
//!   // 5c. classes_rt! [ ... ]
//!   [
//!     "(?:\\b\\w+::)?classes_rt!\\s*\\[((?:[^\\[\\]]|\\[[^\\[\\]]*\\])*?)\\]",
//!     "\"([^\"\\\\]*(?:\\\\.[^\"\\\\]*)*)\"|'([^'\\\\]*(?:\\\\.[^'\\\\]*)*)'|r#{0,3}\"([\\s\\S]*?)\"#{0,3}|\\b(?:[A-Za-z_]\\w*::)*[A-Z][A-Z0-9_]*\\b"
//!   ]
//! ]
//! ```
//!
//! # Advice for consistency
//!
//! Order of the rules is good for maintainability and readability
//!
//! 01. Layout/Position/Overflow
//! 02. Flexbox/Grid/Alignment
//! 03. Spacing (p/m/space)
//! 04. Sizing (w/h/min/max)
//! 05. Typography (font/text/leading/tracking/list/placeholder)
//! 06. Backgrounds
//! 07. Borders/Radii
//! 08. Effects (shadow/opacity)
//! 09. Tables
//! 10. Transitions/Animation
//! 11. Transforms
//! 12. Interactivity
//! 13. SVG
//! 14. Accessibility
//!
//! Author: aav

/// Private module for re-exporting [`const_str`]
pub mod __private {
    pub use const_str;
}

#[macro_export]
/// Combines multiple Tailwind classes into a single space-separated string
///
/// This macro uses `const_str::concat!` for compile-time string concatenation,
/// resulting in zero runtime overhead
///
/// # Examples
///
/// ```rust
/// let button_classes = tw::classes!(
///     "bg-blue-500",
///     "inline-flex",
///     "items-center",
///     "justify-center",
///     "text-white",
///     "px-4",
///     "py-2",
///     "rounded-md",
/// );
/// assert_eq!(button_classes, "bg-blue-500 inline-flex items-center justify-center text-white px-4 py-2 rounded-md");
/// ```
macro_rules! classes {
    // Single expression, optional trailing comma
    ($first:expr $(,)?) => { $first };

    // Two or more expressions, optional trailing comma
    ($first:expr, $($rest:expr),+ $(,)?) => {
        // ::tw::__private::const_str::concat!($first $(, " ", $rest)+) // <-- if this is standalone crate
        $crate::classes::__private::const_str::concat!($first $(, " ", $rest)+)
    };
}

/// Returns `if_true` when `condition` is `true`, otherwise `if_false`
///
/// This is a `const fn` so it can be used in other compile-time contexts
/// and never allocates
///
/// # Examples
///
/// ```rust
/// const PRIMARY: &str = tw::class_if(false, "bg-blue-500", "bg-gray-300");
/// assert_eq!(PRIMARY, "bg-gray-300");
/// ```
pub const fn class_if(
    condition: bool,
    if_true: &'static str,
    if_false: &'static str,
) -> &'static str {
    if condition { if_true } else { if_false }
}

#[macro_export]
/// Build a single space-separated class string at runtime using a macro
///
/// Syntax: required classes first, then a semicolon, then any number of
/// conditional guards of the form `(cond) => "class"`. All tokens are
/// plain `&str` so editor tooling that scans string literals still works
///
/// Allocates one `String` and skips any conditional entries whose guards are
/// false. Accepts optional trailing commas in both sections
///
/// # Examples
///
/// ```rust
/// let active = true;
/// let s = tw::classes_rt!(
///     "inline-flex", "items-center", "text-red-200";
///     (active) => "bg-blue-500 border-blue-500",
///     (!active) => "bg-gray-300 border-gray-300",
/// );
/// assert_eq!(s, "inline-flex items-center text-red-200 bg-blue-500 border-blue-500");
/// ```
macro_rules! classes_rt {
    // with required and conditional parts
    ($($req:expr),+ $(,)? ; $(($cond:expr) => $c:expr),* $(,)?) => {{
        let mut result = String::new();
        // required
        $(
            if !result.is_empty() { result.push(' '); }
            result.push_str($req);
        )*
        // conditional
        $(
            if $cond {
                if !result.is_empty() { result.push(' '); }
                result.push_str($c);
            }
        )*
        result
    }};

    // only required parts, no conditionals
    ($($req:expr),+ $(,)?) => {{
        ::tw::classes!($($req),* $(,)?)
    }};
}
