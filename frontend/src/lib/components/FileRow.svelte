<script lang="ts">
	import { Folder, File } from 'lucide-svelte';
	import type { FileItem } from '$lib/types';

	/**
	 * @prop item - the file or directory entry to render
	 * @prop selected - whether this row is currently checked/selected
	 * @prop onselect - callback invoked with the item when the checkbox changes or a file is clicked
	 * @prop onnavigate - callback invoked with the target path when a directory is clicked
	 */
	let {
		item,
		selected = false,
		onselect,
		onnavigate,
	}: {
		item: FileItem;
		selected?: boolean;
		onselect: (item: FileItem) => void;
		onnavigate: (path: string) => void;
	} = $props();

	/**
	 * Formats a raw byte count into a human-readable size string.
	 *
	 * Returns an empty string for null (size unknown), otherwise scales
	 * the value to the most appropriate unit: B, KB, MB, or GB.
	 *
	 * @param bytes - raw file size in bytes, or null if unknown
	 */
	function formatSize(bytes: number | null): string {
		// --------------------------------------------------
		// return empty string for unknown sizes
		// --------------------------------------------------
		if (bytes === null) return '';
		// --------------------------------------------------
		// scale to the largest unit that keeps the value >= 1
		// --------------------------------------------------
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
		return `${(bytes / 1073741824).toFixed(1)} GB`;
	}

	/**
	 * Handles a click on the file row button.
	 *
	 * Directories trigger navigation into that path; regular files
	 * trigger selection so they can be included in a link.
	 */
	function handleClick() {
		// --------------------------------------------------
		// navigate into directories, select regular files
		// --------------------------------------------------
		if (item.is_dir) {
			onnavigate(item.path);
		} else {
			onselect(item);
		}
	}
</script>

<div
	class="flex items-center gap-3 px-4 py-2.5 rounded-lg transition-colors cursor-pointer
		{selected ? 'bg-accent-muted border border-accent/30' : 'hover:bg-surface-hover border border-transparent'}"
>
	<input
		type="checkbox"
		checked={selected}
		onchange={() => onselect(item)}
		class="w-4 h-4 rounded border-border accent-accent"
		onclick={(e) => e.stopPropagation()}
	/>
	<button onclick={handleClick} class="flex items-center gap-3 flex-1 min-w-0 text-left">
		{#if item.is_dir}
			<Folder size={18} class="text-accent shrink-0" />
		{:else}
			<File size={18} class="text-text-muted shrink-0" />
		{/if}
		<span class="truncate text-text text-sm">{item.name}</span>
		{#if !item.is_dir && item.size !== null}
			<span class="ml-auto text-xs text-text-muted shrink-0">{formatSize(item.size)}</span>
		{/if}
	</button>
</div>
