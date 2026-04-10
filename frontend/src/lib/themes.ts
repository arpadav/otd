// --------------------------------------------------
// theme palette registry - defines all built-in theme presets as
// light/dark color maps. each color maps directly to an
// `--otd-<key>` CSS variable applied on `document.documentElement`.
//
// Author: aav
// --------------------------------------------------
// types
// --------------------------------------------------
/**
 * Full set of CSS custom property values for a single theme mode (light or dark)
 *
 * Each key maps directly to an `--otd-<key>` CSS variable applied on
 * `document.documentElement` by `applyTheme`
 */
export interface ThemeColors {
    surface: string;
    "surface-alt": string;
    "surface-hover": string;
    "surface-active": string;
    border: string;
    "border-light": string;
    text: string;
    "text-muted": string;
    "text-inverse": string;
    accent: string;
    "accent-hover": string;
    "accent-muted": string;
    success: string;
    warning: string;
    danger: string;
    info: string;
}

/**
 * Paired light and dark color sets for a single named theme
 */
export interface ThemeVariant {
    [mode: string]: ThemeColors;
}

// --------------------------------------------------
// theme definitions
// --------------------------------------------------
/**
 * All built-in theme variants keyed by theme name
 *
 * Each entry provides a `light` and `dark` `ThemeColors` map. The keys here
 * must match the names exported in `THEME_NAMES` from `constants.ts`, which
 * is the shared source of truth with the Rust backend
 */
export const themes: Record<ThemeName, ThemeVariant> = {
    forest: {
        light: {
            surface: "#f5f0eb",
            "surface-alt": "#ffffff",
            "surface-hover": "#ebe5dd",
            "surface-active": "#e0d8cc",
            border: "#d4cbbf",
            "border-light": "#e8e0d5",
            text: "#2c3529",
            "text-muted": "#6b7a66",
            "text-inverse": "#f5f0eb",
            accent: "#4a7c59",
            "accent-hover": "#3d6a4a",
            "accent-muted": "#e8f0ea",
            success: "#4a7c59",
            warning: "#b5862a",
            danger: "#b54a4a",
            info: "#4a6a7c",
        },
        dark: {
            surface: "#1a2018",
            "surface-alt": "#232b20",
            "surface-hover": "#2c3529",
            "surface-active": "#354032",
            border: "#3d4a38",
            "border-light": "#2f3a2b",
            text: "#e2ddd5",
            "text-muted": "#9ca896",
            "text-inverse": "#1a2018",
            accent: "#6aad7b",
            "accent-hover": "#7dbf8e",
            "accent-muted": "#2a3d2e",
            success: "#6aad7b",
            warning: "#d4a84a",
            danger: "#d46a6a",
            info: "#6a9ab5",
        },
    },
    clay: {
        light: {
            surface: "#f7f2ee",
            "surface-alt": "#ffffff",
            "surface-hover": "#efe8e1",
            "surface-active": "#e6ddd3",
            border: "#d9cec2",
            "border-light": "#ebe3da",
            text: "#3b2f26",
            "text-muted": "#8a7568",
            "text-inverse": "#f7f2ee",
            accent: "#b5704a",
            "accent-hover": "#a3613e",
            "accent-muted": "#f2e4da",
            success: "#5e8a5e",
            warning: "#c49240",
            danger: "#c45050",
            info: "#5e7a8a",
        },
        dark: {
            surface: "#201a16",
            "surface-alt": "#2a231d",
            "surface-hover": "#352c24",
            "surface-active": "#40352c",
            border: "#4d3f34",
            "border-light": "#382f27",
            text: "#e8dfd7",
            "text-muted": "#b09e90",
            "text-inverse": "#201a16",
            accent: "#d4956a",
            "accent-hover": "#e0a87e",
            "accent-muted": "#3d2e22",
            success: "#7aad7a",
            warning: "#d4a84a",
            danger: "#d46a6a",
            info: "#7a9eb5",
        },
    },
    ocean: {
        light: {
            surface: "#f0f4f5",
            "surface-alt": "#ffffff",
            "surface-hover": "#e4ebee",
            "surface-active": "#d8e2e6",
            border: "#c2d1d8",
            "border-light": "#dae5ea",
            text: "#1e3038",
            "text-muted": "#5a7a88",
            "text-inverse": "#f0f4f5",
            accent: "#2d7d9a",
            "accent-hover": "#256a84",
            "accent-muted": "#dceef4",
            success: "#4a8a6a",
            warning: "#b5862a",
            danger: "#b54a4a",
            info: "#2d7d9a",
        },
        dark: {
            surface: "#141e22",
            "surface-alt": "#1c282e",
            "surface-hover": "#253238",
            "surface-active": "#2e3d44",
            border: "#384d55",
            "border-light": "#283a42",
            text: "#d5e0e4",
            "text-muted": "#88a8b5",
            "text-inverse": "#141e22",
            accent: "#4eb5d4",
            "accent-hover": "#64c4e0",
            "accent-muted": "#1e3640",
            success: "#6aad88",
            warning: "#d4a84a",
            danger: "#d46a6a",
            info: "#4eb5d4",
        },
    },
    stone: {
        light: {
            surface: "#f3f2f0",
            "surface-alt": "#ffffff",
            "surface-hover": "#e8e6e3",
            "surface-active": "#dddad6",
            border: "#ccc8c2",
            "border-light": "#e2dfd9",
            text: "#2e2c28",
            "text-muted": "#78746d",
            "text-inverse": "#f3f2f0",
            accent: "#6b6560",
            "accent-hover": "#5a5450",
            "accent-muted": "#e8e5e2",
            success: "#5e8a6a",
            warning: "#b5862a",
            danger: "#b54a4a",
            info: "#5a7088",
        },
        dark: {
            surface: "#1a1918",
            "surface-alt": "#232220",
            "surface-hover": "#2e2c2a",
            "surface-active": "#383634",
            border: "#454240",
            "border-light": "#333130",
            text: "#e0ddd8",
            "text-muted": "#a09b94",
            "text-inverse": "#1a1918",
            accent: "#a09a94",
            "accent-hover": "#b0aaa3",
            "accent-muted": "#333130",
            success: "#7aad88",
            warning: "#d4a84a",
            danger: "#d46a6a",
            info: "#7a98b5",
        },
    },
    sage: {
        light: {
            surface: "#f2f4f0",
            "surface-alt": "#ffffff",
            "surface-hover": "#e6eae2",
            "surface-active": "#dae0d4",
            border: "#c8d0c0",
            "border-light": "#dee4d8",
            text: "#2a2e26",
            "text-muted": "#6e7a66",
            "text-inverse": "#f2f4f0",
            accent: "#7a9470",
            "accent-hover": "#6a8260",
            "accent-muted": "#e4ece0",
            success: "#6a946a",
            warning: "#b5922a",
            danger: "#b54a4a",
            info: "#5a7a8a",
        },
        dark: {
            surface: "#181a16",
            "surface-alt": "#20231e",
            "surface-hover": "#2a2e26",
            "surface-active": "#34382e",
            border: "#404538",
            "border-light": "#2e3328",
            text: "#dce0d6",
            "text-muted": "#96a08e",
            "text-inverse": "#181a16",
            accent: "#96b58a",
            "accent-hover": "#a8c49e",
            "accent-muted": "#283020",
            success: "#88b588",
            warning: "#d4b04a",
            danger: "#d46a6a",
            info: "#7a9eb5",
        },
    },
    dusk: {
        light: {
            surface: "#f3f0f5",
            "surface-alt": "#ffffff",
            "surface-hover": "#e8e4ec",
            "surface-active": "#ddd8e2",
            border: "#cec6d6",
            "border-light": "#e2dce8",
            text: "#2c2832",
            "text-muted": "#74688a",
            "text-inverse": "#f3f0f5",
            accent: "#7d6a9a",
            "accent-hover": "#6c5a88",
            "accent-muted": "#ece6f2",
            success: "#5e8a6a",
            warning: "#c49240",
            danger: "#b54a4a",
            info: "#6a7a9a",
        },
        dark: {
            surface: "#1a181e",
            "surface-alt": "#222028",
            "surface-hover": "#2c2832",
            "surface-active": "#36323e",
            border: "#443e50",
            "border-light": "#302c38",
            text: "#e0dce4",
            "text-muted": "#a098b0",
            "text-inverse": "#1a181e",
            accent: "#a896c4",
            "accent-hover": "#baaad4",
            "accent-muted": "#302840",
            success: "#7aad88",
            warning: "#d4b04a",
            danger: "#d46a6a",
            info: "#8a9ac4",
        },
    },
};

// --------------------------------------------------
// exports and utilities
// --------------------------------------------------
import { THEME_NAMES, type ThemeName, type ThemeMode } from "$lib/constants";

/**
 * Re-export of the shared `THEME_NAMES` constant for use in theme-related UI
 *
 * Sourced from the backend-synchronized `constants.json` via `$lib/constants`
 */
export const themeNames = Object.keys(THEME_NAMES);

/**
 * Applies a named theme and mode to the document root as CSS custom properties
 *
 * Iterates over all color keys in the resolved `ThemeColors` map and sets each
 * as `--otd-<key>` on `document.documentElement`. No-ops silently if the theme
 * name is not found in the `themes` registry
 *
 * @param name - The theme name to apply (must be a key in `themes`)
 * @param mode - Whether to apply the `light` or `dark` variant
 */
export function applyTheme(name: ThemeName, mode: ThemeMode): void {
    // --------------------------------------------------
    // look up the theme variant; bail early if unknown
    // --------------------------------------------------
    const variant = themes[name];
    if (!variant) return;
    // --------------------------------------------------
    // resolve the color map for the requested mode and
    // write each value as a CSS custom property on :root
    // --------------------------------------------------
    const colors = variant[mode];
    const root = document.documentElement;
    for (const [key, value] of Object.entries(colors)) {
        root.style.setProperty(`--otd-${key}`, value);
    }
}
