// --------------------------------------------------
// state
// --------------------------------------------------
/**
 * Reactive authentication state object.
 *
 * `loggedIn` is the single source of truth for whether a valid session exists.
 * `ready` flips to `true` after the initial auth probe in `+layout.svelte`
 * resolves, so child routes can avoid rendering or firing API calls before
 * we know whether the user is authenticated. Components that read these
 * values inside a `$derived` or reactive block re-render automatically when
 * the setters mutate the object.
 */
const auth = $state({ loggedIn: false, ready: false });

/**
 * Getter: Returns whether the user currently has an active session.
 *
 * @returns `true` if the user is authenticated, `false` otherwise
 */
export function getLoggedIn(): boolean {
    return auth.loggedIn;
}

/**
 * Setter: Sets the authenticated state of the current user.
 *
 * Mutates the reactive `auth` state object, which triggers a Svelte re-render
 * on all components that read `getLoggedIn()`.  Pass `true` after a successful
 * login response and `false` on logout or session expiry.
 *
 * @param value - `true` to mark the user as logged in, `false` to mark them as logged out
 */
export function setLoggedIn(value: boolean): void {
    // --------------------------------------------------
    // update reactive auth state (triggers Svelte re-renders)
    // --------------------------------------------------
    auth.loggedIn = value;
}

/**
 * Getter: Returns whether the initial auth probe has completed.
 *
 * Routes that gate rendering on knowing the auth state should read this
 * and show a loading shell until it flips to `true`.
 */
export function getReady(): boolean {
    return auth.ready;
}

/**
 * Setter: Marks the initial auth probe as complete.
 *
 * Called by the root layout after `api.getMe()` resolves (or rejects),
 * which lets gated routes proceed with rendering or redirect to `/login`.
 *
 * @param value - `true` to mark the probe complete
 */
export function setReady(value: boolean): void {
    auth.ready = value;
}
