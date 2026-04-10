<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { onMount } from 'svelte';
	import { Link, Trash2, LinkIcon } from 'lucide-svelte';
	import { api } from '$lib/api';
	import { addToast } from '$lib/stores/toast.svelte';
	import LinkRow from '$lib/components/LinkRow.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import type { TokenListItem, UpdateLinkRequest } from '$lib/types';
	import { LINK_STATUSES, BULK_DELETE_FILTERS } from '$lib/constants';

	// --------------------------------------------------
	// state
	// --------------------------------------------------
	/** full list of download links returned by the last listLinks API call */
	let links = $state<TokenListItem[]>([]);
	/** true during the initial fetch - drives skeleton placeholder rows */
	let loading = $state(true);
	/** controls visibility of the bulk-delete confirmation dialog */
	let confirmOpen = $state(false);
	/** API filter string passed to the bulk-delete endpoint (e.g. "used", "expired") */
	let bulkFilter = $state('');
	/** human-readable description of what will be deleted, shown in the confirm dialog */
	let bulkLabel = $state('');

	// --------------------------------------------------
	// derived
	// --------------------------------------------------
	/**
	 * True when at least one link has an archive currently being prepared
	 * Used to decide the polling interval - fast (2 s) when preparing,
	 * slow (30 s) otherwise - so the UI stays in sync without hammering the API
	 */
	let hasPreparing = $derived(links.some((l) => l.link_status === LINK_STATUSES.preparing));
	/**
	 * Count of links that have been fully exhausted (0 remaining downloads)
	 * but have not yet expired. Drives visibility of the "Clear Used" button
	 */
	let usedCount = $derived(links.filter((l) => l.remaining_downloads <= 0 && !l.expired).length);
	/**
	 * Count of links whose expiry date has passed. Drives visibility of the
	 * "Clear Expired" button
	 */
	let expiredCount = $derived(links.filter((l) => l.expired).length);

	// --------------------------------------------------
	// functions
	// --------------------------------------------------
	/**
	 * Loads the full list of download links from the API and replaces the
	 * local `links` state
	 *
	 * Called on mount and on every polling tick. Suppresses "Unauthorized"
	 * errors because those are handled globally by the API client (redirect
	 * to login). All other errors trigger an error toast
	 */
	async function fetchLinks() {
		// --------------------------------------------------
		// fetch the link list and replace local state; the
		// finally block always clears the initial loading flag
		// --------------------------------------------------
		try {
			links = await api.listLinks();
		} catch (e) {
			// --------------------------------------------------
			// ignore auth errors - global handler redirects to
			// login; surface all other failures as a toast
			// --------------------------------------------------
			if (e instanceof Error && e.message !== 'Unauthorized') {
				addToast('Failed to load links', 'error');
			}
		} finally {
			// --------------------------------------------------
			// clear loading on first fetch; subsequent polling
			// calls are silent (no skeleton shown again)
			// --------------------------------------------------
			loading = false;
		}
	}

	/**
	 * Deletes a single download link by token and removes it from the local
	 * list optimistically without a full refetch
	 *
	 * @param token - The unique token identifying the link to delete
	 */
	async function handleDelete(token: string) {
		// --------------------------------------------------
		// call the delete API, then remove the link from the
		// local list to avoid a full refetch round-trip
		// --------------------------------------------------
		try {
			await api.deleteLink(token);
			links = links.filter((l) => l.token !== token);
			addToast('Link deleted', 'success');
		} catch {
			// --------------------------------------------------
			// deletion failed - keep the link in the list and
			// show an error so the user can retry
			// --------------------------------------------------
			addToast('Failed to delete link', 'error');
		}
	}

	/**
	 * Edits an existing link's max_downloads and expiry by calling the API
	 * and replacing the local entry with the server's response
	 *
	 * @param token - The unique token identifying the link to edit
	 * @param req - The updated max_downloads and expires_in_seconds values
	 */
	async function handleEdit(token: string, req: UpdateLinkRequest) {
		try {
			const updated = await api.editLink(token, req);
			// --------------------------------------------------
			// replace the local entry with the server's response
			// --------------------------------------------------
			links = links.map((l) => (l.token === token ? updated : l));
			addToast('Link updated', 'success');
		} catch {
			addToast('Failed to update link', 'error');
		}
	}

	/**
	 * Triggers a background archive rebuild for a link whose archive has
	 * been evicted or corrupted, then refreshes the link list
	 *
	 * After the revive request succeeds the link's `link_status` will
	 * transition to preparing, which causes the polling interval to drop to
	 * 2 s so the UI reflects completion quickly
	 *
	 * @param token - The unique token identifying the link to revive
	 */
	async function handleRevive(token: string) {
		// --------------------------------------------------
		// request archive rebuild from the API
		// --------------------------------------------------
		try {
			await api.reviveLink(token);
			addToast('Archive rebuild started', 'info');
			// --------------------------------------------------
			// refresh list so the updated link_status is shown
			// immediately (PREPARING state triggers fast polling)
			// --------------------------------------------------
			await fetchLinks();
		} catch (e) {
			// --------------------------------------------------
			// surface server error message if available
			// --------------------------------------------------
			addToast(e instanceof Error ? e.message : 'Failed to revive link', 'error');
		}
	}

	/**
	 * Prepares and opens the bulk-delete confirmation dialog for a given
	 * filter category
	 *
	 * Stores the filter key and a human-readable label so the confirmation
	 * message and the subsequent API call both use consistent values
	 *
	 * @param filter - API filter key (e.g. BULK_DELETE_FILTERS.used)
	 * @param label  - Human-readable description shown in the confirm dialog
	 */
	function openBulkDelete(filter: string, label: string) {
		// --------------------------------------------------
		// store the filter and label for use by confirmBulkDelete
		// once the user confirms the dialog
		// --------------------------------------------------
		bulkFilter = filter;
		bulkLabel = label;
		confirmOpen = true;
	}

	/**
	 * Executes the bulk delete using the filter stored by `openBulkDelete`,
	 * toasts the number of removed links, then refreshes the link list
	 *
	 * Called by ConfirmDialog's `onconfirm` callback after the user confirms
	 */
	async function confirmBulkDelete() {
		// --------------------------------------------------
		// send the bulk-delete request with the stored filter
		// --------------------------------------------------
		try {
			const res = await api.bulkDeleteLinks(bulkFilter);
			// --------------------------------------------------
			// pluralise the toast message based on how many links
			// were actually removed
			// --------------------------------------------------
			addToast(`Deleted ${res.removed} link${res.removed === 1 ? '' : 's'}`, 'success');
			// --------------------------------------------------
			// refresh the list to reflect the deletions
			// --------------------------------------------------
			await fetchLinks();
		} catch {
			addToast('Bulk delete failed', 'error');
		}
	}

	// --------------------------------------------------
	// lifecycle
	// --------------------------------------------------
	onMount(() => fetchLinks());

	// --------------------------------------------------
	// adaptive polling: 2 s while any archive is preparing,
	// 30 s otherwise. the $effect re-creates the interval
	// whenever hasPreparing changes so the rate adapts
	// automatically without a page refresh
	// --------------------------------------------------
	$effect(() => {
		const ms = hasPreparing ? 2000 : 30000;
		const interval = setInterval(fetchLinks, ms);
		return () => clearInterval(interval);
	});
</script>

<svelte:head>
	<title>Links - OTD</title>
</svelte:head>

<div class="max-w-6xl mx-auto px-4 py-8">
	<div class="flex items-center justify-between mb-6">
		<div>
			<h1 class="text-2xl font-bold text-text">Links</h1>
			<p class="text-text-muted text-sm mt-1">{links.length} total link{links.length === 1 ? '' : 's'}</p>
		</div>
		<div class="flex items-center gap-2">
			{#if usedCount > 0}
				<button
					onclick={() => openBulkDelete(BULK_DELETE_FILTERS.used, `${usedCount} used link${usedCount === 1 ? '' : 's'}`)}
					class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm border border-border
						text-text-muted hover:text-danger hover:border-danger/30 transition-colors"
				>
					<Trash2 size={14} />
					Clear Used
				</button>
			{/if}
			{#if expiredCount > 0}
				<button
					onclick={() => openBulkDelete(BULK_DELETE_FILTERS.expired, `${expiredCount} expired link${expiredCount === 1 ? '' : 's'}`)}
					class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm border border-border
						text-text-muted hover:text-danger hover:border-danger/30 transition-colors"
				>
					<Trash2 size={14} />
					Clear Expired
				</button>
			{/if}
		</div>
	</div>

	{#if loading}
		<div class="space-y-2">
			{#each Array(4) as _}
				<div class="h-16 rounded-lg bg-surface-alt border border-border animate-pulse"></div>
			{/each}
		</div>
	{:else if links.length === 0}
		<div class="text-center py-16 text-text-muted">
			<LinkIcon size={40} class="mx-auto mb-3 opacity-40" />
			<p>No download links yet</p>
			<a href="/browse" class="inline-block mt-3 text-sm text-accent hover:text-accent-hover transition-colors">
				Browse files to create one
			</a>
		</div>
	{:else}
		<div class="space-y-2">
			{#each links as link (link.token)}
				<LinkRow {link} ondelete={handleDelete} onrevive={handleRevive} onedit={handleEdit} />
			{/each}
		</div>
	{/if}
</div>

<ConfirmDialog
	bind:open={confirmOpen}
	title="Delete Links"
	message="This will permanently delete {bulkLabel}. This action cannot be undone."
	confirmLabel="Delete"
	onconfirm={confirmBulkDelete}
/>
