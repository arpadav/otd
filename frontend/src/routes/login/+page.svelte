<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { goto } from '$app/navigation';
	import { Lock } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { setLoggedIn } from '$lib/stores/auth.svelte';
	import { addToast } from '$lib/stores/toast.svelte';

	// --------------------------------------------------
	// state
	// --------------------------------------------------
	/** Bound value of the password input field */
	let password = $state('');
	/** True while the login request is in flight - disables the submit button */
	let loading = $state(false);
	/** Inline error message shown below the password field on failure */
	let error = $state('');

	// --------------------------------------------------
	// handlers
	// --------------------------------------------------
	/**
	 * Handles login form submission.
	 *
	 * Prevents the native form POST, sends the password to the auth API,
	 * and on success updates the global auth state before redirecting to
	 * the dashboard. On failure the error is shown inline rather than via
	 * a toast, keeping the feedback local to the form.
	 *
	 * @param e - The native submit event (used only to call preventDefault)
	 */
	async function handleSubmit(e: Event) {
		// --------------------------------------------------
		// prevent native form submission and reset state
		// --------------------------------------------------
		e.preventDefault();
		loading = true;
		error = '';
		// --------------------------------------------------
		// attempt authentication via the API
		// --------------------------------------------------
		try {
			const res = await api.login(password);
			if (res.success) {
				// --------------------------------------------------
				// update global auth state and redirect to dashboard
				// --------------------------------------------------
				setLoggedIn(true);
				await goto('/');
			} else {
				// --------------------------------------------------
				// show inline error for invalid credentials
				// --------------------------------------------------
				error = 'Invalid password';
			}
		} catch {
			// --------------------------------------------------
			// handle network or unexpected errors
			// --------------------------------------------------
			error = 'Login failed. Please try again.';
		} finally {
			// --------------------------------------------------
			// always clear the loading indicator
			// --------------------------------------------------
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Login - OTD</title>
</svelte:head>

<div class="flex items-center justify-center min-h-screen px-4">
	<div class="w-full max-w-sm">
		<div class="text-center mb-8">
			<div class="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-accent-muted text-accent mb-4">
				<Lock size={28} />
			</div>
			<h1 class="text-2xl font-bold text-text">OTD</h1>
			<p class="text-text-muted text-sm mt-1">One-Time Downloads</p>
		</div>

		<form onsubmit={handleSubmit} class="bg-surface-alt rounded-xl border border-border p-6 space-y-4">
			<div>
				<label for="password" class="block text-sm font-medium text-text mb-1.5">Admin Password</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					placeholder="Enter password"
					required
					class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text placeholder-text-muted
						focus:outline-none focus:ring-2 focus:ring-accent/40 focus:border-accent transition-colors"
				/>
			</div>

			{#if error}
				<p class="text-danger text-sm">{error}</p>
			{/if}

			<button
				type="submit"
				disabled={loading || !password}
				class="w-full py-2.5 rounded-lg bg-accent text-text-inverse font-medium
					hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
			>
				{loading ? 'Signing in...' : 'Sign In'}
			</button>
		</form>
	</div>
</div>
