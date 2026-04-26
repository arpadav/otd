<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import { onMount } from "svelte";
    import { api } from "$lib/api";
    import { addToast } from "$lib/stores/toast.svelte";
    import {
        getThemeName,
        getThemeMode,
        setTheme,
    } from "$lib/stores/theme.svelte";
    import { THEME_MODES } from "$lib/constants";
    import { themeNames } from "$lib/themes";
    import ThemeSwatch from "$lib/components/ThemeSwatch.svelte";
    import { Sun, Moon } from "lucide-svelte";

    // --------------------------------------------------
    // settings state
    // --------------------------------------------------
    let downloadBaseUrl = $state("");
    let downloadBaseUrlLoaded = $state(false);
    let oldPassword = $state("");
    let newPassword = $state("");
    let confirmPassword = $state("");
    let savingUrl = $state(false);
    let savingPassword = $state(false);

    // --------------------------------------------------
    // load settings on mount
    // --------------------------------------------------
    onMount(() => {
        api.getSettings()
            .then((s) => {
                downloadBaseUrl = s.download_base_url ?? "";
                downloadBaseUrlLoaded = true;
            })
            .catch(() => {
                addToast("Failed to load settings", "error");
            });
    });

    // --------------------------------------------------
    // settings handlers
    // --------------------------------------------------
    /**
     * Saves the download base URL to the server
     */
    async function saveDownloadUrl() {
        savingUrl = true;
        try {
            const result = await api.updateSettings({
                download_base_url: downloadBaseUrl.trim() || null,
            });
            downloadBaseUrl = result.download_base_url ?? "";
            addToast("Download URL updated", "success");
        } catch {
            addToast("Failed to save download URL", "error");
        } finally {
            savingUrl = false;
        }
    }

    /**
     * Changes the admin password.
     *
     * The server invalidates every other session and issues the caller a
     * fresh session cookie in the response, so the user stays logged in
     * here while every other tab/device is forced to re-authenticate.
     */
    async function handleChangePassword() {
        if (newPassword !== confirmPassword) {
            addToast("Passwords do not match", "error");
            return;
        }
        if (!newPassword) {
            addToast("New password cannot be empty", "error");
            return;
        }
        savingPassword = true;
        try {
            await api.changePassword({
                old_password: oldPassword,
                new_password: newPassword,
            });
            oldPassword = "";
            newPassword = "";
            confirmPassword = "";
            addToast("Password changed successfully", "success");
        } catch {
            addToast("Failed to change password", "error");
        } finally {
            savingPassword = false;
        }
    }

    // --------------------------------------------------
    // theme handlers
    // --------------------------------------------------
    /**
     * Applies a new theme by name while preserving the current light/dark mode,
     * then persists the selection to the server
     *
     * The local store is updated immediately (optimistic) so the UI reflects
     * the change without waiting for the API round-trip. If the API call fails
     * the local theme stays changed - the server and client may drift, but the
     * UX is more responsive
     *
     * @param name - Theme name to activate (must be a key in `themeNames`)
     */
    async function selectTheme(name: string) {
        // --------------------------------------------------
        // read the current mode so it is preserved when only
        // the theme name changes
        // --------------------------------------------------
        const mode = getThemeMode();
        // --------------------------------------------------
        // apply the change locally first for instant feedback
        // --------------------------------------------------
        setTheme(name, mode);
        // --------------------------------------------------
        // persist the selection to the server; failure only
        // shows a toast - the local state is already updated
        // --------------------------------------------------
        try {
            await api.setTheme({ name, mode });
        } catch {
            addToast("Failed to save theme", "error");
        }
    }

    /**
     * Toggles between light and dark mode while preserving the current theme
     * name, then persists the new mode to the server
     *
     * Like `selectTheme`, this is optimistic - the local store is updated
     * synchronously before the API call completes
     */
    async function handleToggleMode() {
        // --------------------------------------------------
        // compute the new mode by flipping the current one
        // --------------------------------------------------
        const newMode =
            getThemeMode() === THEME_MODES.light
                ? THEME_MODES.dark
                : THEME_MODES.light;
        // --------------------------------------------------
        // read the current theme name so it is preserved when
        // only the mode changes
        // --------------------------------------------------
        const name = getThemeName();
        // --------------------------------------------------
        // apply locally first for instant visual feedback
        // --------------------------------------------------
        setTheme(name, newMode);
        // --------------------------------------------------
        // persist the new mode to the server
        // --------------------------------------------------
        try {
            await api.setTheme({ name, mode: newMode });
        } catch {
            addToast("Failed to save theme", "error");
        }
    }
</script>

<svelte:head>
    <title>Settings - OTD</title>
</svelte:head>

<div class="max-w-6xl mx-auto px-4 py-8">
    <div class="mb-8">
        <h1 class="text-2xl font-bold text-text">Settings</h1>
        <p class="text-text-muted text-sm mt-1">
            Customize your OTD experience
        </p>
    </div>

    <div class="bg-surface-alt rounded-xl border border-border p-6 mb-6">
        <div class="flex items-center justify-between mb-6">
            <div>
                <h2 class="text-lg font-semibold text-text">Appearance</h2>
                <p class="text-sm text-text-muted mt-0.5">
                    Choose a theme and color mode
                </p>
            </div>
            <button
                onclick={handleToggleMode}
                class="flex items-center gap-2 px-4 py-2 rounded-lg border border-border
					text-text-muted hover:text-text hover:bg-surface-hover transition-colors text-sm"
            >
                {#if getThemeMode() === THEME_MODES.dark}
                    <Sun size={16} />
                    Light Mode
                {:else}
                    <Moon size={16} />
                    Dark Mode
                {/if}
            </button>
        </div>

        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
            {#each themeNames as name}
                <ThemeSwatch
                    {name}
                    active={getThemeName() === name}
                    mode={getThemeMode()}
                    onselect={selectTheme}
                />
            {/each}
        </div>
    </div>

    <div class="bg-surface-alt rounded-xl border border-border p-6 mb-6">
        <h2 class="text-lg font-semibold text-text mb-1">Download URL</h2>
        <p class="text-sm text-text-muted mb-4">
            Custom base URL for generated download links. Leave empty to derive
            from server host/port.
        </p>
        {#if downloadBaseUrlLoaded}
            <form
                onsubmit={(e) => {
                    e.preventDefault();
                    saveDownloadUrl();
                }}
                class="flex gap-3"
            >
                <input
                    type="text"
                    bind:value={downloadBaseUrl}
                    placeholder="https://files.example.com"
                    class="flex-1 px-3 py-2 rounded-lg border border-border bg-surface text-text
						text-sm placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent"
                />
                <button
                    type="submit"
                    disabled={savingUrl}
                    class="px-4 py-2 rounded-lg bg-accent text-white text-sm font-medium
						hover:bg-accent-hover transition-colors disabled:opacity-50"
                >
                    {savingUrl ? "Saving..." : "Save"}
                </button>
            </form>
        {/if}
    </div>

    <div class="bg-surface-alt rounded-xl border border-border p-6 mb-6">
        <h2 class="text-lg font-semibold text-text mb-1">Change Password</h2>
        <p class="text-sm text-text-muted mb-4">
            Set or update the admin password. You will stay logged in here;
            other tabs and devices will need to sign in again.
        </p>
        <form
            onsubmit={(e) => {
                e.preventDefault();
                handleChangePassword();
            }}
            class="space-y-3 max-w-md"
        >
            <input
                type="password"
                bind:value={oldPassword}
                placeholder="Current password"
                class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
					text-sm placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent"
            />
            <input
                type="password"
                bind:value={newPassword}
                placeholder="New password"
                class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
					text-sm placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent"
            />
            <input
                type="password"
                bind:value={confirmPassword}
                placeholder="Confirm new password"
                class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
					text-sm placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent"
            />
            <button
                type="submit"
                disabled={savingPassword}
                class="px-4 py-2 rounded-lg bg-accent text-white text-sm font-medium
					hover:bg-accent-hover transition-colors disabled:opacity-50"
            >
                {savingPassword ? "Changing..." : "Change Password"}
            </button>
        </form>
    </div>
</div>
