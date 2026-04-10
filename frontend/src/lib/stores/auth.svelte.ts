// --------------------------------------------------
// state
// --------------------------------------------------
/**
 * Reactive authentication state object.
 *
 * `loggedIn` is the single source of truth for whether a valid session exists.
 * Components that read `getLoggedIn()` inside a `$derived` or reactive block
 * will re-render automatically whenever `setLoggedIn` mutates this value.
 */
const auth = $state({ loggedIn: false });

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
