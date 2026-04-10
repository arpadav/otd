<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { LINK_STATUSES, type LinkStatus } from '$lib/constants';

	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/**
	 * @prop status - the current link status to display as a colored badge
	 */
	let {
		status,
	}: {
		status: LinkStatus;
	} = $props();

	// --------------------------------------------------
	// constants
	// --------------------------------------------------
	/** maps each status value to its tailwind color classes */
	const styles: Record<string, string> = {
		[LINK_STATUSES.active]: 'bg-success/15 text-success',
		[LINK_STATUSES.used]: 'bg-info/15 text-info',
		[LINK_STATUSES.expired]: 'bg-warning/15 text-warning',
		[LINK_STATUSES.preparing]: 'bg-accent-muted text-accent',
		[LINK_STATUSES.failed]: 'bg-danger/15 text-danger',
	};
	/** maps each status value to its human-readable display label */
	const labels: Record<string, string> = {
		[LINK_STATUSES.active]: 'Active',
		[LINK_STATUSES.used]: 'Used',
		[LINK_STATUSES.expired]: 'Expired',
		[LINK_STATUSES.preparing]: 'Preparing',
		[LINK_STATUSES.failed]: 'Failed',
	};
</script>

<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {styles[status]}">
	{labels[status]}
</span>
