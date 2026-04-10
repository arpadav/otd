<script lang="ts">
    import { ChevronRight, House } from "lucide-svelte";

    /**
     * @prop path - the current directory path as a slash-separated string
     * @prop onnavigate - callback invoked with the target path when a segment is clicked
     */
    let {
        path,
        onnavigate,
    }: {
        path: string;
        onnavigate: (path: string) => void;
    } = $props();

    // derived list of breadcrumb segments, each carrying its display name and
    // the cumulative path prefix needed to navigate back to that directory level
    let segments = $derived(
        path
            .split("/")
            .filter(Boolean)
            .map((seg, i, arr) => ({
                name: seg,
                path: arr.slice(0, i + 1).join("/"),
            })),
    );
</script>

<nav class="flex items-center gap-1 text-sm text-text-muted">
    <button
        onclick={() => onnavigate("")}
        class="hover:text-text transition-colors p-1 rounded hover:bg-surface-hover"
    >
        <House size={16} />
    </button>
    {#each segments as segment}
        <ChevronRight size={14} class="opacity-40" />
        <button
            onclick={() => onnavigate(segment.path)}
            class="hover:text-text transition-colors px-1.5 py-0.5 rounded hover:bg-surface-hover truncate max-w-48"
        >
            {segment.name}
        </button>
    {/each}
</nav>
