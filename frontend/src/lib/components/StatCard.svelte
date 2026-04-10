<script lang="ts">
	/* eslint-disable @typescript-eslint/no-explicit-any */
	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/**
	 * @prop label - Descriptive text shown beneath the value (e.g. "Total Files")
	 * @prop value - The numeric or string statistic to display prominently
	 * @prop icon - Lucide icon component rendered inside the colored badge
	 * @prop color - Tailwind text-color class applied to the icon badge (defaults to accent)
	 */
	let {
		label,
		value,
		icon,
		color = 'text-accent',
	}: {
		label: string;
		value: string | number;
		icon: any;
		color?: string;
	} = $props();
</script>

<div class="bg-surface-alt rounded-xl border border-border p-5 flex items-center gap-4">
	<div class="p-3 rounded-lg bg-surface-hover {color}">
		{#if icon}
			{@const Icon = icon}
			<Icon size={22} />
		{/if}
	</div>
	<div>
		<p class="text-2xl font-bold text-text">{value}</p>
		<p class="text-sm text-text-muted">{label}</p>
	</div>
</div>
