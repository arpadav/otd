<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { fade, scale } from 'svelte/transition';
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/**
	 * @prop open - Controls modal visibility (two-way bindable)
	 * @prop title - Optional heading rendered at the top of the modal panel
	 * @prop children - Slot content rendered inside the modal body
	 */
	let {
		open = $bindable(false),
		title = '',
		children,
	}: {
		open: boolean;
		title?: string;
		children: Snippet;
	} = $props();

	// --------------------------------------------------
	// handlers
	// --------------------------------------------------
	/**
	 * Closes the modal when the user clicks outside the panel.
	 *
	 * Bound to the backdrop overlay element. Clicks on the panel itself
	 * are stopped via `e.stopPropagation()` so they do not bubble here.
	 */
	function onBackdropClick() {
		open = false;
	}

	/**
	 * Closes the modal when the Escape key is pressed.
	 *
	 * Bound to `svelte:window` so it fires regardless of which element
	 * currently holds focus.
	 *
	 * @param e - The keyboard event from `svelte:window`
	 */
	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-40 flex items-center justify-center bg-black/40"
		transition:fade={{ duration: 150 }}
		onclick={onBackdropClick}
		onkeydown={onKeydown}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="bg-surface-alt rounded-xl shadow-xl border border-border w-full max-w-md mx-4 p-6"
			transition:scale={{ start: 0.95, duration: 200 }}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			{#if title}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold text-text">{title}</h2>
					<button
						onclick={() => (open = false)}
						class="text-text-muted hover:text-text transition-colors p-1 rounded-lg hover:bg-surface-hover"
					>
						<X size={18} />
					</button>
				</div>
			{/if}
			{@render children()}
		</div>
	</div>
{/if}
