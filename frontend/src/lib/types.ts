// --------------------------------------------------
// shared TypeScript interface definitions for all API request and
// response shapes used by the OTD frontend - covers file browser
// entries, stats, link management, theme preferences, auth, and
// error envelopes.
//
// Author: aav
// --------------------------------------------------
// file browser types
// --------------------------------------------------
import type { LinkStatus, ThemeName, ThemeMode } from "./constants";

/**
 * A single entry returned by the file browser API
 *
 * `size` is `null` for directories, since directory sizes are not computed
 * server-side. `path` is the full server-side path used in subsequent API
 * calls (e.g. generating a link or browsing into a subdirectory)
 */
export interface FileItem {
    /** Display name of the file or directory */
    name: string;
    /** Full server-side path, used as the argument to browse or link generation */
    path: string;
    /** Whether this entry is a directory (`true`) or a regular file (`false`) */
    is_dir: boolean;
    /** File size in bytes, or `null` for directories */
    size: number | null;
}

// --------------------------------------------------
// stats types
// --------------------------------------------------
/**
 * Aggregate statistics about the OTD server returned by `/api/stats`
 *
 * Counts reflect the current in-memory state of the link store; they are
 * not persisted across restarts
 */
export interface StatsResponse {
    /** Number of links that are still active and have remaining downloads */
    active_links: number;
    /** Number of links that have been fully consumed (zero downloads remaining) */
    used_links: number;
    /** Number of links whose expiry timestamp has passed */
    expired_links: number;
    /** Cumulative download count across all links since server start */
    total_downloads: number;
    /** How long the server has been running, in seconds */
    uptime_seconds: number;
}

// --------------------------------------------------
// link management types
// --------------------------------------------------
/**
 * A single link entry as returned by the `/api/links` list endpoint
 *
 * Includes both static metadata (token, paths, limits) and dynamic runtime
 * state (download count, remaining downloads, expiry, archive status)
 */
export interface TokenListItem {
    /** The URL-safe token that identifies this link */
    token: string;
    /** Human-readable label assigned at link creation time */
    name: string;
    /** Whether this link covers multiple files (will be served as an archive) */
    is_multi_file: boolean;
    /** How many times this link has been downloaded so far */
    download_count: number;
    /** Maximum number of downloads allowed (0 means unlimited) */
    max_downloads: number;
    /** Downloads remaining before the link is exhausted */
    remaining_downloads: number;
    /** Whether the link has passed its expiry time */
    expired: boolean;
    /** Seconds until expiry, or `null` if the link has no expiry */
    expires_in_seconds: number | null;
    /** Full URL that can be shared to trigger a download */
    download_url: string;
    /** Server-side file paths covered by this link */
    paths: string[];
    /** Current link status (see `LINK_STATUSES` constants) */
    link_status: LinkStatus;
    /** Whether the source file(s) still exist on disk */
    source_exists: boolean;
}

/**
 * Request body for creating a new download link via `/api/links`
 */
export interface GenerateRequest {
    /** One or more server-side file paths to include in the link */
    paths: string[];
    /** Optional human-readable label for the link */
    name?: string;
    /** Maximum number of times the link may be downloaded (omit for unlimited) */
    max_downloads?: number;
    /** Link lifetime in seconds from creation (omit for no expiry) */
    expires_in_seconds?: number;
    /** Desired archive format when multiple paths are included (e.g. "zip") */
    format?: string;
}

/**
 * Response body returned after successfully creating a new download link
 */
export interface GenerateResponse {
    /** The newly created URL-safe token */
    token: string;
    /** The full URL that can be shared to trigger a download */
    download_url: string;
}

/**
 * Response body returned by delete and bulk-delete link endpoints
 */
export interface BulkDeleteResponse {
    /** Total number of links that were removed by the operation */
    removed: number;
}

// --------------------------------------------------
// theme and auth types
// --------------------------------------------------
/**
 * A user's persisted theme preference, stored and retrieved via `/api/theme`
 */
export interface ThemePreference {
    /** Theme name (must match a key in the `themes` registry) */
    name: ThemeName;
    /** Color mode from `THEME_MODES` */
    mode: ThemeMode;
}

/**
 * Request body for the `/api/auth/login` endpoint
 */
export interface LoginRequest {
    /** The plain-text password to authenticate with */
    password: string;
}

/**
 * Response body returned by login and logout endpoints
 */
export interface LoginResponse {
    /** Whether the auth operation succeeded */
    success: boolean;
}

// --------------------------------------------------
// settings types
// --------------------------------------------------
/**
 * Response body returned by `GET /api/settings`
 */
export interface SettingsResponse {
    /** Custom download base URL, or `null` if derived from CLI host/port */
    download_base_url: string | null;
}

/**
 * Request body for `PUT /api/settings`
 */
export interface UpdateSettingsRequest {
    /** New download base URL, or `null` to reset to default */
    download_base_url: string | null;
}

/**
 * Request body for `POST /api/settings/password`
 */
export interface ChangePasswordRequest {
    /** Current admin password (empty string if no password is set) */
    old_password: string;
    /** New admin password */
    new_password: string;
}

/**
 * Request body for `PUT /api/links/{token}`
 */
export interface UpdateLinkRequest {
    /** New maximum number of downloads allowed (minimum 1) */
    max_downloads: number;
    /** New expiry in seconds from now, or `null` for no expiry */
    expires_in_seconds: number | null;
}

/**
 * Shape of error responses returned by the API on non-OK status codes
 */
export interface ApiError {
    /** Human-readable error message from the server */
    error: string;
}
