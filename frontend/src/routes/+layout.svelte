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
	import { setLoggedIn } from '$lib/stores/auth.svelte';
	import { page } from '$app/state';

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

	// --------------------------------------------------
	// lifecycle
	// --------------------------------------------------
	/**
	 * Initialises theme and auth state on mount.
	 *
	 * Attempts to load the user's saved theme preference from the API.
	 * Falls back to the system colour-scheme preference if the request fails
	 * or the user is unauthenticated. Then probes the stats endpoint to
	 * determine whether a valid session already exists and sets the global
	 * auth state accordingly.
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
		// probe auth state via the stats endpoint
		// --------------------------------------------------
		try {
			await api.getStats();
			setLoggedIn(true);
		} catch {
			setLoggedIn(false);
		}
	});
</script>

<div class="min-h-screen flex flex-col bg-surface text-text">
	{#if !isLoginPage}
		<Navbar />
	{/if}
	<main class="flex-1">
		{@render children()}
	</main>
	{#if !isLoginPage}
		<Footer />
	{/if}
	<Toast />
</div>
