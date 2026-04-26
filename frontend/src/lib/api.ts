// --------------------------------------------------
// imports
// --------------------------------------------------
import { goto } from '$app/navigation';
import type {
	StatsResponse,
	FileItem,
	TokenListItem,
	GenerateRequest,
	GenerateResponse,
	BulkDeleteResponse,
	ThemePreference,
	LoginResponse,
	SettingsResponse,
	UpdateSettingsRequest,
	ChangePasswordRequest,
	UpdateLinkRequest,
} from './types';

// --------------------------------------------------
// request helper
// --------------------------------------------------
/**
 * Sends an authenticated HTTP request and deserializes the JSON response.
 *
 * Attaches `Content-Type: application/json` and `credentials: same-origin` to
 * every request. Redirects to `/login` and throws on 401. Throws a descriptive
 * error for any other non-OK status, using the `error` field from the response
 * body when available.
 *
 * @param url - The endpoint URL to request
 * @param options - Optional `fetch` init overrides (method, body, headers, etc.)
 * @returns The parsed JSON body typed as `T`
 */
async function request<T>(
	url: string,
	options?: RequestInit & { silent401?: boolean },
): Promise<T> {
	// --------------------------------------------------
	// pull the silent401 flag out of the merged options so
	// it doesn't get forwarded to fetch as an unknown init
	// --------------------------------------------------
	const { silent401, ...fetchOptions } = options ?? {};
	// --------------------------------------------------
	// send the fetch request with default credentials
	// and content-type headers merged over any overrides
	// --------------------------------------------------
	const res = await fetch(url, {
		credentials: 'same-origin',
		...fetchOptions,
		headers: {
			'Content-Type': 'application/json',
			...fetchOptions?.headers,
		},
	});
	// --------------------------------------------------
	// redirect to login on 401 - session is invalid or
	// the user was never authenticated. callers that need
	// to handle 401 themselves (e.g. the auth probe) pass
	// silent401: true to opt out of the auto-redirect
	// --------------------------------------------------
	if (res.status === 401) {
		if (!silent401) {
			await goto('/login');
		}
		throw new Error('Unauthorized');
	}
	// --------------------------------------------------
	// surface a descriptive error for any other failure,
	// preferring the JSON body's `error` field over the
	// raw HTTP status text
	// --------------------------------------------------
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(body.error ?? res.statusText);
	}
	return res.json();
}

// --------------------------------------------------
// api endpoints
// --------------------------------------------------
/**
 * Typed client for all OTD backend API endpoints.
 *
 * Every method delegates to `request<T>`, which handles auth redirects,
 * error surfacing, and JSON (de)serialization. Callers only need to handle
 * domain-level errors - network and auth failures are managed centrally.
 */
export const api = {
	/**
	 * Authenticates the user with the given password.
	 *
	 * @param password - The plain-text password to verify against the server
	 * @returns A `LoginResponse` indicating whether authentication succeeded
	 */
	login(password: string): Promise<LoginResponse> {
		return request('/api/auth/login', {
			method: 'POST',
			body: JSON.stringify({ password }),
		});
	},

	/**
	 * Invalidates the current session and logs the user out.
	 *
	 * @returns A `LoginResponse` (success field will be true on clean logout)
	 */
	logout(): Promise<LoginResponse> {
		return request('/api/auth/logout', { method: 'POST' });
	},

	/**
	 * Probes the server for current session state.
	 *
	 * Returns whether the request carries a valid session and whether the
	 * server has a password configured at all. Uses `silent401` so that an
	 * unauthenticated probe does NOT trigger the auto-redirect in the
	 * request helper - the caller decides what to do with the result.
	 *
	 * @returns `{ logged_in, password_required }`
	 */
	getMe(): Promise<{ logged_in: boolean; password_required: boolean }> {
		return request('/api/auth/me', { silent401: true });
	},

	/**
	 * Retrieves the user's persisted theme preference from the server.
	 *
	 * @returns The stored `ThemePreference` (name + mode)
	 */
	getTheme(): Promise<ThemePreference> {
		return request('/api/theme');
	},

	/**
	 * Persists a new theme preference to the server.
	 *
	 * @param pref - The theme name and mode to save
	 * @returns The updated `ThemePreference` as confirmed by the server
	 */
	setTheme(pref: ThemePreference): Promise<ThemePreference> {
		return request('/api/theme', {
			method: 'PUT',
			body: JSON.stringify(pref),
		});
	},

	/**
	 * Fetches aggregate statistics about links managed by the server.
	 *
	 * @returns A `StatsResponse` with counts for active, used, expired links
	 *   and the total download count plus server uptime
	 */
	getStats(): Promise<StatsResponse> {
		return request('/api/stats');
	},

	/**
	 * Lists the contents of a server-side directory.
	 *
	 * Omitting `path` (or passing an empty string) returns the root listing.
	 * The path is URL-encoded before being sent as a query parameter.
	 *
	 * @param path - Server-side directory path to list; defaults to root
	 * @returns An array of `FileItem` entries in the requested directory
	 */
	browse(path: string = ''): Promise<FileItem[]> {
		const params = path ? `?path=${encodeURIComponent(path)}` : '';
		return request(`/api/browse${params}`);
	},

	/**
	 * Retrieves all active download links visible to the authenticated user.
	 *
	 * @returns An array of `TokenListItem` entries with token metadata and status
	 */
	listLinks(): Promise<TokenListItem[]> {
		return request('/api/links');
	},

	/**
	 * Creates a new download link for one or more file paths.
	 *
	 * @param req - The link generation request (paths, optional name, expiry, etc.)
	 * @returns A `GenerateResponse` containing the new token and its download URL
	 */
	generateLink(req: GenerateRequest): Promise<GenerateResponse> {
		return request('/api/links', {
			method: 'POST',
			body: JSON.stringify(req),
		});
	},

	/**
	 * Updates the max_downloads and expiry of an existing link.
	 *
	 * @param token - The URL-safe token identifying the link to edit
	 * @param req - The fields to update
	 * @returns The updated `TokenListItem`
	 */
	editLink(token: string, req: UpdateLinkRequest): Promise<TokenListItem> {
		return request(`/api/links/${encodeURIComponent(token)}`, {
			method: 'PUT',
			body: JSON.stringify(req),
		});
	},

	/**
	 * Deletes a single download link by token.
	 *
	 * @param token - The URL-safe token identifying the link to remove
	 * @returns A `BulkDeleteResponse` with the count of removed links (always 1)
	 */
	deleteLink(token: string): Promise<BulkDeleteResponse> {
		return request(`/api/links/${encodeURIComponent(token)}`, {
			method: 'DELETE',
		});
	},

	/**
	 * Revives an expired or exhausted download link, resetting its state.
	 *
	 * @param token - The URL-safe token identifying the link to revive
	 * @returns Resolves with no value on success
	 */
	reviveLink(token: string): Promise<void> {
		return request(`/api/links/${encodeURIComponent(token)}/revive`, {
			method: 'POST',
		});
	},

	/**
	 * Bulk-deletes download links matching a filter criterion.
	 *
	 * Valid filter values are defined in `BULK_DELETE_FILTERS` (used, expired, all).
	 * The filter is URL-encoded before being passed as a query parameter.
	 *
	 * @param filter - One of the `BulkDeleteFilter` string values
	 * @returns A `BulkDeleteResponse` with the total count of removed links
	 */
	bulkDeleteLinks(filter: string): Promise<BulkDeleteResponse> {
		return request(`/api/links?filter=${encodeURIComponent(filter)}`, {
			method: 'DELETE',
		});
	},

	/**
	 * Retrieves the current persistent server settings.
	 *
	 * @returns A `SettingsResponse` with the download base URL
	 */
	getSettings(): Promise<SettingsResponse> {
		return request('/api/settings');
	},

	/**
	 * Updates persistent server settings.
	 *
	 * @param req - The settings to update
	 * @returns The updated `SettingsResponse`
	 */
	updateSettings(req: UpdateSettingsRequest): Promise<SettingsResponse> {
		return request('/api/settings', {
			method: 'PUT',
			body: JSON.stringify(req),
		});
	},

	/**
	 * Changes the admin password.
	 *
	 * Verifies the old password, hashes the new one, and clears all sessions.
	 *
	 * @param req - Old and new password
	 * @returns Resolves on success
	 */
	changePassword(req: ChangePasswordRequest): Promise<{ success: boolean }> {
		return request('/api/settings/password', {
			method: 'POST',
			body: JSON.stringify(req),
		});
	},
};
