<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import {
        Sun,
        Moon,
        FolderOpen,
        Link,
        LayoutDashboard,
        Settings,
        LogOut,
        Menu,
        X,
        Info,
    } from "lucide-svelte";
    import { getThemeMode, toggleMode } from "$lib/stores/theme.svelte";
    import { THEME_MODES } from "$lib/constants";
    import { getLoggedIn, setLoggedIn } from "$lib/stores/auth.svelte";
    import { api } from "$lib/api";
    import { addToast } from "$lib/stores/toast.svelte";

    // --------------------------------------------------
    // state
    // --------------------------------------------------
    /** Controls whether the mobile nav drawer is expanded */
    let mobileOpen = $state(false);

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /** Top-level navigation entries rendered in both desktop and mobile nav */
    const links = [
        { href: "/", label: "Dashboard", icon: LayoutDashboard },
        { href: "/browse", label: "Files", icon: FolderOpen },
        { href: "/links", label: "Links", icon: Link },
        { href: "/settings", label: "Settings", icon: Settings },
        { href: "/about", label: "About", icon: Info },
    ];

    // --------------------------------------------------
    // derived
    // --------------------------------------------------
    /** Current URL pathname - used to highlight the active nav link */
    let currentPath = $derived(page.url.pathname);

    // --------------------------------------------------
    // handlers
    // --------------------------------------------------
    /**
     * Logs the current user out and redirects to the login page
     *
     * Calls the API logout endpoint, clears the auth store, then navigates
     * to `/login`. Displays an error toast if the request fails
     */
    async function handleLogout() {
        // --------------------------------------------------
        // attempt logout via API, then clear local auth state
        // --------------------------------------------------
        try {
            await api.logout();
            setLoggedIn(false);
            await goto("/login");
        } catch {
            // --------------------------------------------------
            // notify the user if the logout request failed
            // --------------------------------------------------
            addToast("Failed to log out", "error");
        }
    }
</script>

<nav class="relative border-b border-border bg-surface-alt">
    <div class="max-w-6xl mx-auto px-4">
        <div class="flex items-center justify-between h-14">
            <a href="/" class="text-lg font-bold text-accent tracking-tight"
                >OTD</a
            >

            <!-- Desktop nav: icons+labels at lg, icons-only at md -->
            <div class="hidden sm:flex items-center gap-1">
                {#each links as link}
                    <a
                        href={link.href}
                        title={link.label}
                        class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-colors
							{currentPath === link.href
                            ? 'bg-accent-muted text-accent font-medium'
                            : 'text-text-muted hover:text-text hover:bg-surface-hover'}"
                    >
                        {#if link.icon}
                            {@const Icon = link.icon}
                            <Icon size={16} />
                        {/if}
                        <span class="hidden lg:inline">{link.label}</span>
                    </a>
                {/each}
            </div>

            <div class="flex items-center gap-2">
                <button
                    onclick={toggleMode}
                    class="p-2 rounded-lg text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
                    title="Toggle dark mode"
                >
                    {#if getThemeMode() === THEME_MODES.dark}
                        <Sun size={18} />
                    {:else}
                        <Moon size={18} />
                    {/if}
                </button>

                {#if getLoggedIn()}
                    <button
                        onclick={handleLogout}
                        title="Logout"
                        class="hidden sm:flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
                    >
                        <LogOut size={16} />
                        <span class="hidden lg:inline">Logout</span>
                    </button>
                {/if}

                <!-- Mobile menu button -->
                <button
                    onclick={() => (mobileOpen = !mobileOpen)}
                    class="sm:hidden p-2 rounded-lg text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
                >
                    {#if mobileOpen}
                        <X size={20} />
                    {:else}
                        <Menu size={20} />
                    {/if}
                </button>
            </div>
        </div>
    </div>

    <!-- Mobile nav dropdown (overlay, does not expand the navbar) -->
    {#if mobileOpen}
        <div
            class="sm:hidden absolute right-0 top-full z-50 w-48 mr-4 mt-1 py-2 space-y-1
			rounded-xl border border-border bg-surface-alt shadow-lg"
        >
            {#each links as link}
                <a
                    href={link.href}
                    onclick={() => (mobileOpen = false)}
                    class="flex items-center gap-2 px-3 py-2 text-sm transition-colors
						{currentPath === link.href
                        ? 'bg-accent-muted text-accent font-medium'
                        : 'text-text-muted hover:text-text hover:bg-surface-hover'}"
                >
                    {#if link.icon}
                        {@const MobileIcon = link.icon}
                        <MobileIcon size={16} />
                    {/if}
                    {link.label}
                </a>
            {/each}
            {#if getLoggedIn()}
                <button
                    onclick={() => {
                        mobileOpen = false;
                        handleLogout();
                    }}
                    class="flex items-center gap-2 px-3 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover transition-colors w-full"
                >
                    <LogOut size={16} />
                    Logout
                </button>
            {/if}
        </div>
    {/if}
</nav>
