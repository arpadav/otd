<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import '../app.css';
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { api } from '$lib/api';
	import { initTheme, initFromSystem } from '$lib/stores/theme.svelte';
	import {
		getLoggedIn,
		getReady,
		setLoggedIn,
		setReady,
	} from '$lib/stores/auth.svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';

	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/** Slot content rendered inside the layout shell */
	let { children }: { children: Snippet } = $props();

	// --------------------------------------------------
	// derived
	// --------------------------------------------------
	/** True when the current route is the login page - used to hide the navbar and footer */
	let isLoginPage = $derived(page.url.pathname === '/login');

	/** True once the initial probe has resolved - gates rendering of child routes */
	let ready = $derived(getReady());

	/**
	 * True if the auth state allows the current route to render.
	 *
	 * The login page renders immediately (no probe needed); every other
	 * route waits for the probe and then renders only when the user is
	 * confirmed authenticated, otherwise the `$effect` below redirects.
	 */
	let canRender = $derived(isLoginPage || (ready && getLoggedIn()));

	// --------------------------------------------------
	// reactive: redirect unauthenticated users away from
	// protected routes once the probe has completed
	// --------------------------------------------------
	$effect(() => {
		if (ready && !getLoggedIn() && !isLoginPage) {
			goto('/login');
		}
	});

	// --------------------------------------------------
	// lifecycle
	// --------------------------------------------------
	/**
	 * Initialises theme and auth state on mount.
	 *
	 * Loads the user's saved theme (falling back to system preference) and
	 * probes `/auth/me` to determine whether a valid session already exists,
	 * marking the auth store as ready when the probe resolves so gated child
	 * routes can render or be redirected.
	 */
	onMount(async () => {
		// --------------------------------------------------
		// load theme: prefer server-side preference, fall back to system
		// --------------------------------------------------
		try {
			const pref = await api.getTheme();
			initTheme(pref);
		} catch {
			initFromSystem();
		}
		// --------------------------------------------------
		// probe auth state via the dedicated /me endpoint
		// --------------------------------------------------
		try {
			const me = await api.getMe();
			setLoggedIn(me.logged_in);
		} catch {
			setLoggedIn(false);
		} finally {
			setReady(true);
		}
	});
</script>

<div class="min-h-screen flex flex-col bg-surface text-text">
	{#if !isLoginPage}
		<Navbar />
	{/if}
	<main class="flex-1">
		{#if canRender}
			{@render children()}
		{/if}
	</main>
	{#if !isLoginPage}
		<Footer />
	{/if}
	<Toast />
</div>
