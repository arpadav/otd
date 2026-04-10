<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { themes } from '$lib/themes';
	import { Check } from 'lucide-svelte';
	import { DEFAULT_THEME_MODE, type ThemeMode } from '$lib/constants';

	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/**
	 * @prop name - the theme identifier key used to look up color values in the themes map
	 * @prop active - whether this swatch represents the currently selected theme
	 * @prop mode - color scheme mode determining which palette variant to display
	 * @prop onselect - callback invoked with the theme name when the swatch is clicked
	 */
	let {
		name,
		active = false,
		mode = DEFAULT_THEME_MODE,
		onselect,
	}: {
		name: string;
		active?: boolean;
		mode?: ThemeMode;
		onselect: (name: string) => void;
	} = $props();

	// --------------------------------------------------
	// derived
	// --------------------------------------------------
	/** the resolved color palette for the current theme name and mode;
	 * undefined when the theme key does not exist in the themes map */
	let colors = $derived(themes[name]?.[mode]);
</script>

{#if colors}
	<button
		onclick={() => onselect(name)}
		class="relative rounded-xl border-2 p-4 transition-all
			{active ? 'border-accent shadow-md' : 'border-border hover:border-border-light hover:shadow-sm'}"
		style="background-color: {colors.surface}"
	>
		{#if active}
			<div class="absolute top-2 right-2 rounded-full p-0.5" style="background-color: {colors.accent}; color: {colors['text-inverse']}">
				<Check size={12} />
			</div>
		{/if}
		<div class="flex gap-1.5 mb-3">
			<div class="w-6 h-6 rounded-md" style="background-color: {colors.accent}"></div>
			<div class="w-6 h-6 rounded-md" style="background-color: {colors['surface-alt']}; border: 1px solid {colors.border}"></div>
			<div class="w-6 h-6 rounded-md" style="background-color: {colors['surface-hover']}"></div>
		</div>
		<p class="text-xs font-medium capitalize text-left" style="color: {colors.text}">{name}</p>
	</button>
{/if}
