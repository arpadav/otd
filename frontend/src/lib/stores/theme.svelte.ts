// --------------------------------------------------
// reactive theme store - tracks the active theme preset and color
// mode, and keeps the document's CSS custom properties in sync.
// exposes getters, setters, and initialization helpers consumed
// by the navbar, settings page, and app bootstrap.
//
// Author: aav
// --------------------------------------------------
// imports
// --------------------------------------------------
import { applyTheme } from '$lib/themes';
import { DEFAULT_THEME_NAME, DEFAULT_THEME_MODE, THEME_MODES } from '$lib/constants';
import type { ThemeMode, ThemeName } from '$lib/constants';
import type { ThemePreference } from '$lib/types';

// --------------------------------------------------
// state
// --------------------------------------------------
/**
 * Reactive theme state object
 *
 * `name` identifies the active theme preset (e.g. `"forest"`, `"clay"`)
 * `mode` controls the light/dark polarity.  Both fields are initialized from
 * the application-level defaults defined in `$lib/constants`
 */
const theme = $state({ name: DEFAULT_THEME_NAME, mode: DEFAULT_THEME_MODE });

// --------------------------------------------------
// getters
// --------------------------------------------------
/**
 * Returns the name of the currently active theme preset
 *
 * Reading `theme.name` inside a Svelte component or `$derived` expression
 * will subscribe to changes, but calling this function from plain TypeScript
 * gives a plain string snapshot without reactivity
 *
 * @returns The theme preset identifier (e.g. `"forest"`)
 */
export function getThemeName(): ThemeName {
	return theme.name;
}

/**
 * Returns the current color mode
 *
 * @returns The active theme mode string (e.g. `"light"` or `"dark"`)
 */
export function getThemeMode(): ThemeMode {
	return theme.mode;
}

// --------------------------------------------------
// setters
// --------------------------------------------------
/**
 * Updates the active theme name and color mode, then applies the palette
 *
 * Mutates both fields of the reactive `theme` state object so that any Svelte
 * component reading `theme.name` or `theme.mode` re-renders automatically
 * After the state update, `applyTheme` writes the corresponding CSS custom
 * properties onto `document.documentElement` so the new palette takes effect
 * immediately without a page reload
 *
 * @param name - Theme preset identifier (e.g. `"forest"`, `"clay"`)
 * @param mode - Color mode from `THEME_MODES`
 */
export function setTheme(name: ThemeName, mode: ThemeMode): void {
	// --------------------------------------------------
	// update reactive state (triggers Svelte re-renders)
	// --------------------------------------------------
	theme.name = name;
	theme.mode = mode;
	// --------------------------------------------------
	// apply CSS custom properties to the document root
	// --------------------------------------------------
	applyTheme(name, mode);
}

/**
 * Toggles between light and dark mode, preserving the current theme preset
 *
 * Reads the current `theme.mode`, flips it, then delegates to `setTheme` so
 * that both the reactive state and the CSS custom properties are updated in
 * one consistent call
 */
export function toggleMode(): void {
	// --------------------------------------------------
	// compute the opposite mode and re-apply the theme
	// --------------------------------------------------
	const newMode = theme.mode === THEME_MODES.light ? THEME_MODES.dark : THEME_MODES.light;
	setTheme(theme.name, newMode);
}

// --------------------------------------------------
// initialization
// --------------------------------------------------
/**
 * Initializes the theme from a persisted user preference
 *
 * Called on application load when a previously saved `ThemePreference` is
 * available (e.g. from the server session or local storage).  Normalizes the
 * mode value before delegating to `setTheme` to ensure only valid values
 * enter the reactive state
 *
 * @param pref - The stored theme preference containing `name` and `mode` fields
 */
export function initTheme(pref: ThemePreference): void {
	// --------------------------------------------------
	// normalize mode to a valid value, then apply
	// --------------------------------------------------
	const mode = pref.mode === THEME_MODES.dark ? THEME_MODES.dark : THEME_MODES.light;
	setTheme(pref.name, mode);
}

/**
 * Initializes the color mode from the OS-level `prefers-color-scheme` media query
 *
 * Used as a fallback when no persisted preference is available.  Guards against
 * SSR environments by checking for `window` before accessing `matchMedia`.  If
 * the system preference is dark, switches the current theme to dark mode while
 * keeping the existing theme preset unchanged
 */
export function initFromSystem(): void {
	// --------------------------------------------------
	// guard against SSR - window is undefined on the server
	// --------------------------------------------------
	if (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
		// --------------------------------------------------
		// system prefers dark - apply dark mode, keep preset
		// --------------------------------------------------
		setTheme(theme.name, THEME_MODES.dark);
	}
}
