// --------------------------------------------------
// shared frontend constants derived from the backend-synchronized
// `constants.json` file - provides typed namespaces and union types
// for theme names, theme modes, link statuses, archive formats,
// expiry units, and bulk-delete filters.
//
// Author: aav
// --------------------------------------------------
// shared constants (single source of truth with Rust backend)
// --------------------------------------------------
import raw from "$shared/constants.json";

// --------------------------------------------------
// helper
// --------------------------------------------------
/** Converts a JSON string array into a namespace object where keys equal values */
function toNamespace(arr: readonly string[]): Record<string, string> {
    return Object.fromEntries(arr.map((s) => [s, s]));
}

// --------------------------------------------------
// theme constants
// --------------------------------------------------
/** Theme names as a namespace, derived from the JSON array */
export const THEME_NAMES = toNamespace(raw.theme.name);
/** Union of all valid theme name strings */
export type ThemeName = (typeof raw.theme.name)[number];
/** The theme name applied when no user preference has been set */
export const DEFAULT_THEME_NAME = raw.theme.name[0];

/** Theme color modes as a namespace, derived from the JSON array */
export const THEME_MODES = toNamespace(raw.theme.mode);
/** Union of all valid theme mode strings */
export type ThemeMode = (typeof raw.theme.mode)[number];
/** The color mode applied when no user preference has been set */
export const DEFAULT_THEME_MODE = raw.theme.mode[0];

// --------------------------------------------------
// link status constants
// --------------------------------------------------
/** Link display statuses as a namespace, derived from the JSON array */
export const LINK_STATUSES = toNamespace(raw.link_statuses);
/** Union of all valid link status strings */
export type LinkStatus = (typeof raw.link_statuses)[number];

// --------------------------------------------------
// archive format constants
// --------------------------------------------------
/** Supported archive formats as a namespace, derived from the JSON array */
export const ARCHIVE_FORMATS = toNamespace(raw.archive_formats);
/** Union of all valid archive format strings */
export type ArchiveFormat = (typeof raw.archive_formats)[number];

// --------------------------------------------------
// expiry unit constants
// --------------------------------------------------
/** Supported expiry time units as a namespace, derived from the JSON array */
export const EXPIRY_UNITS = toNamespace(raw.expiry_units);
/** Union of all valid expiry unit strings */
export type ExpiryUnit = (typeof raw.expiry_units)[number];

// --------------------------------------------------
// expiry multipliers (seconds per unit)
// --------------------------------------------------
/** Maps each expiry unit to its equivalent in seconds */
export const EXPIRY_MULTIPLIERS: Record<string, number> = {
    [EXPIRY_UNITS.minutes]: 60,
    [EXPIRY_UNITS.hours]: 3600,
    [EXPIRY_UNITS.days]: 86400,
    [EXPIRY_UNITS.weeks]: 604800,
    [EXPIRY_UNITS.months]: 2592000,
};

// --------------------------------------------------
// bulk delete filter constants
// --------------------------------------------------
/** Bulk-delete filters as a namespace, derived from the JSON array */
export const BULK_DELETE_FILTERS = toNamespace(raw.bulk_delete_filters);
/** Union of all valid bulk-delete filter strings */
export type BulkDeleteFilter = (typeof raw.bulk_delete_filters)[number];
