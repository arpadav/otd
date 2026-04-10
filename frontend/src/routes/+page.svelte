<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { onMount } from 'svelte';
	import { Link, Download, Clock, AlertTriangle, Activity } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { addToast } from '$lib/stores/toast.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import type { StatsResponse } from '$lib/types';

	// --------------------------------------------------
	// state
	// --------------------------------------------------
	/** Server statistics fetched from the API; null until the first successful response */
	let stats = $state<StatsResponse | null>(null);
	/** True while the initial stats fetch is in flight - drives the skeleton loader */
	let loading = $state(true);

	// --------------------------------------------------
	// functions
	// --------------------------------------------------
	/**
	 * Fetches server statistics from the API and updates local state.
	 *
	 * Unauthorised errors are silently swallowed because they are handled
	 * globally by the layout auth probe. Any other error surfaces as a toast
	 * so the user knows the data may be stale.
	 */
	async function fetchStats() {
		// --------------------------------------------------
		// request stats from the API
		// --------------------------------------------------
		try {
			stats = await api.getStats();
		} catch (e) {
			// --------------------------------------------------
			// suppress auth errors; surface all other failures as a toast
			// --------------------------------------------------
			if (e instanceof Error && e.message !== 'Unauthorized') {
				addToast('Failed to load stats', 'error');
			}
		} finally {
			// --------------------------------------------------
			// always clear the loading skeleton on first load
			// --------------------------------------------------
			loading = false;
		}
	}

	/**
	 * Formats a raw uptime value in seconds into a human-readable string.
	 *
	 * @param seconds - Total uptime in seconds
	 * @returns A compact string such as `2d 3h`, `5h 12m`, or `47m`
	 */
	function formatUptime(seconds: number): string {
		const days = Math.floor(seconds / 86400);
		const hours = Math.floor((seconds % 86400) / 3600);
		const mins = Math.floor((seconds % 3600) / 60);
		if (days > 0) return `${days}d ${hours}h`;
		if (hours > 0) return `${hours}h ${mins}m`;
		return `${mins}m`;
	}

	// --------------------------------------------------
	// lifecycle
	// --------------------------------------------------
	/**
	 * Starts the stats polling loop on mount.
	 *
	 * Performs an immediate fetch, then polls every 30 seconds so the
	 * dashboard stays live without a manual refresh. The interval is
	 * cleared on component destroy via the returned cleanup function.
	 *
	 * @returns Cleanup function that cancels the polling interval
	 */
	onMount(() => {
		// --------------------------------------------------
		// fetch immediately, then poll every 30 seconds
		// --------------------------------------------------
		fetchStats();
		const interval = setInterval(fetchStats, 30000);
		// --------------------------------------------------
		// return cleanup to cancel the interval on destroy
		// --------------------------------------------------
		return () => clearInterval(interval);
	});
</script>

<svelte:head>
	<title>Dashboard - OTD</title>
</svelte:head>

<div class="max-w-6xl mx-auto px-4 py-8">
	<div class="mb-8">
		<h1 class="text-2xl font-bold text-text">Dashboard</h1>
		<p class="text-text-muted text-sm mt-1">One-Time Downloads server overview</p>
	</div>

	{#if loading}
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
			{#each Array(4) as _}
				<div class="bg-surface-alt rounded-xl border border-border p-5 h-24 animate-pulse"></div>
			{/each}
		</div>
	{:else if stats}
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
			<StatCard label="Active Links" value={stats.active_links} icon={Link} color="text-success" />
			<StatCard label="Total Downloads" value={stats.total_downloads} icon={Download} color="text-info" />
			<StatCard label="Used Links" value={stats.used_links} icon={AlertTriangle} color="text-warning" />
			<StatCard label="Uptime" value={formatUptime(stats.uptime_seconds)} icon={Activity} color="text-accent" />
		</div>

		<div class="mt-8 grid grid-cols-1 sm:grid-cols-2 gap-4">
			<a
				href="/browse"
				class="flex items-center gap-4 p-5 rounded-xl border border-border bg-surface-alt hover:bg-surface-hover transition-colors"
			>
				<div class="p-3 rounded-lg bg-accent-muted text-accent">
					<Download size={22} />
				</div>
				<div>
					<p class="font-medium text-text">Browse Files</p>
					<p class="text-sm text-text-muted">Select files and generate download links</p>
				</div>
			</a>
			<a
				href="/links"
				class="flex items-center gap-4 p-5 rounded-xl border border-border bg-surface-alt hover:bg-surface-hover transition-colors"
			>
				<div class="p-3 rounded-lg bg-accent-muted text-accent">
					<Link size={22} />
				</div>
				<div>
					<p class="font-medium text-text">Manage Links</p>
					<p class="text-sm text-text-muted">View, copy, and manage active download links</p>
				</div>
			</a>
		</div>
	{/if}
</div>
