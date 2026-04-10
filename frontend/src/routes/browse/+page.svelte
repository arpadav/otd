<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Search, FolderPlus, Link, X } from "lucide-svelte";
    import { api } from "$lib/api";
    import { addToast } from "$lib/stores/toast.svelte";
    import {
        ARCHIVE_FORMATS,
        EXPIRY_UNITS,
        EXPIRY_MULTIPLIERS,
    } from "$lib/constants";
    import Breadcrumbs from "$lib/components/Breadcrumbs.svelte";
    import FileRow from "$lib/components/FileRow.svelte";
    import Modal from "$lib/components/Modal.svelte";
    import type { FileItem, GenerateRequest } from "$lib/types";

    // --------------------------------------------------
    // state
    // --------------------------------------------------
    /** the directory path currently displayed in the file browser */
    let currentPath = $state("");
    /** the full list of file items returned by the last browse API call */
    let items = $state<FileItem[]>([]);
    /** set of file paths the user has checked for link generation */
    let selected = $state<Set<string>>(new Set());
    /** true while a browse API request is in flight - drives skeleton UI */
    let loading = $state(true);
    /** current value of the search input - filters `items` into `filtered` */
    let search = $state("");

    /** controls visibility of the generate-link modal */
    let generateOpen = $state(false);
    /** optional human-readable name for the generated download link */
    let genName = $state("");
    /** optional cap on how many times the link may be downloaded */
    let genMaxDownloads = $state<number | undefined>(undefined);
    /** numeric amount for the expiry duration */
    let genExpiryAmount = $state(1);
    /** time unit for the expiry duration */
    let genExpiryUnit = $state(EXPIRY_UNITS.weeks);
    /** when true, link never expires */
    let genNoExpiry = $state(false);
    /** selected archive format for multi-file links */
    let genFormat = $state(ARCHIVE_FORMATS.zip);
    /** true while the generate-link API call is in flight - disables submit */
    let generating = $state(false);
    /** whether the staged files section in the modal is expanded */
    let stagedExpanded = $state(true);

    // --------------------------------------------------
    // derived
    // --------------------------------------------------
    /**
     * Items visible in the file list after applying the search filter
     * When `search` is empty the full `items` array is returned as-is to
     * avoid an unnecessary allocating filter pass
     */
    let filtered = $derived(
        search
            ? items.filter((i) =>
                  i.name.toLowerCase().includes(search.toLowerCase()),
              )
            : items,
    );
    /** number of currently selected files - drives button label and guard */
    let selectedCount = $derived(selected.size);

    // --------------------------------------------------
    // alias for readability
    // --------------------------------------------------
    const expiryMultipliers = EXPIRY_MULTIPLIERS;

    // --------------------------------------------------
    // display labels for archive formats
    // --------------------------------------------------
    /** maps each archive format key to its display label */
    const formatLabels: Record<string, string> = {
        [ARCHIVE_FORMATS.zip]: "ZIP",
        [ARCHIVE_FORMATS.tar_gz]: "TAR.GZ",
        [ARCHIVE_FORMATS.tar]: "TAR",
    };

    /**
     * Fetches and displays the contents of a directory
     *
     * Updates the current path, resets selection state, and toggles loading
     * On success, replaces `items` with the directory listing from the API
     * Suppresses "Unauthorized" errors; all other errors trigger a toast notification
     *
     * @param path - Absolute or relative path to browse
     */
    async function browse(path: string) {
        // --------------------------------------------------
        // update the current directory path being viewed
        // --------------------------------------------------
        currentPath = path;
        // --------------------------------------------------
        // indicate that a browse request is in progress
        // --------------------------------------------------
        loading = true;
        // --------------------------------------------------
        // fetch directory contents from the API and replace
        // the current item list (selection is preserved
        // across directory changes)
        // --------------------------------------------------
        try {
            items = await api.browse(path);
        } catch (e) {
            // --------------------------------------------------
            // suppress auth errors; surface all other failures
            // --------------------------------------------------
            if (e instanceof Error && e.message !== "Unauthorized") {
                addToast("Failed to browse directory", "error");
            }
        } finally {
            // --------------------------------------------------
            // always clear loading state after request completes
            // --------------------------------------------------
            loading = false;
        }
    }

    /**
     * Toggles the selection state of a single file item
     *
     * Creates a new Set from the current selection so that Svelte detects the
     * mutation and re-renders the derived `selectedCount` and button label
     *
     * @param item - The file item whose selection state should be toggled
     */
    function toggleSelect(item: FileItem) {
        const next = new Set(selected);
        if (next.has(item.path)) {
            next.delete(item.path);
        } else {
            next.add(item.path);
        }
        selected = next;
    }

    /**
     * Removes a single file from the selection by path
     *
     * Used by the staged files list in the modal to deselect individual items
     *
     * @param path - The file path to remove from selection
     */
    function removeSelected(path: string) {
        const next = new Set(selected);
        next.delete(path);
        selected = next;
    }

    /**
     * Selects all visible (filtered) items, or clears selection if all are
     * already selected - acting as a toggle-all control
     */
    function selectAll() {
        const next = new Set(selected);
        const allVisible = filtered.every((i) => next.has(i.path));
        if (allVisible) {
            for (const i of filtered) next.delete(i.path);
        } else {
            for (const i of filtered) next.add(i.path);
        }
        selected = next;
    }

    /**
     * Opens the generate-link modal after validating that at least one file
     * is selected
     *
     * Resets all modal form fields to their defaults before opening so that a
     * previous submission does not bleed into the next one
     */
    function openGenerate() {
        if (selectedCount === 0) {
            addToast("Select files first", "warning");
            return;
        }
        genName = "";
        genMaxDownloads = undefined;
        genExpiryAmount = 1;
        genExpiryUnit = EXPIRY_UNITS.weeks;
        genNoExpiry = false;
        genFormat = ARCHIVE_FORMATS.zip;
        stagedExpanded = true;
        generateOpen = true;
    }

    /**
     * Computes the expiry duration in seconds from the current form state
     *
     * @returns Seconds until expiry, or `undefined` when no-expiry is checked
     */
    function computeExpirySeconds(): number | undefined {
        if (genNoExpiry) return undefined;
        return genExpiryAmount * (expiryMultipliers[genExpiryUnit] ?? 604800);
    }

    /**
     * Submits the generate-link form, creates the download link via the API,
     * copies the resulting URL to the clipboard, and redirects to the links page
     */
    async function handleGenerate() {
        generating = true;
        try {
            const req: GenerateRequest = {
                paths: [...selected],
                name: genName || undefined,
                max_downloads: genMaxDownloads,
                expires_in_seconds: computeExpirySeconds(),
            };
            // --------------------------------------------------
            // include archive format only for multi-file links
            // --------------------------------------------------
            if (selectedCount > 1) {
                req.format = genFormat;
            }
            const res = await api.generateLink(req);
            await navigator.clipboard.writeText(res.download_url);
            addToast("Link created and copied to clipboard", "success");
            generateOpen = false;
            selected = new Set();
            // --------------------------------------------------
            // redirect to the links page after successful creation
            // --------------------------------------------------
            await goto("/links");
        } catch (e) {
            addToast(
                e instanceof Error ? e.message : "Failed to generate link",
                "error",
            );
        } finally {
            generating = false;
        }
    }

    /**
     * Extracts the filename from a full file path
     *
     * @param path - Full server-side file path
     * @returns The last segment of the path (the filename)
     */
    function filename(path: string): string {
        return path.split("/").pop() ?? path;
    }

    // --------------------------------------------------
    // lifecycle
    // --------------------------------------------------
    onMount(() => browse(""));
</script>

<svelte:head>
    <title>Browse Files - OTD</title>
</svelte:head>

<div class="max-w-6xl mx-auto px-4 py-8">
    <div class="flex items-center justify-between mb-6">
        <div>
            <h1 class="text-2xl font-bold text-text">Files</h1>
            <div class="mt-2">
                <Breadcrumbs path={currentPath} onnavigate={browse} />
            </div>
        </div>
        <button
            onclick={openGenerate}
            disabled={selectedCount === 0}
            class="flex items-center gap-2 px-4 py-2 rounded-lg bg-accent text-text-inverse
				hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
        >
            <Link size={16} />
            Generate Link{selectedCount > 0 ? ` (${selectedCount})` : ""}
        </button>
    </div>

    <div class="flex gap-6">
        <!-- file browser (left) -->
        <div class="flex-1 min-w-0">
            <div class="flex items-center gap-3 mb-4">
                <div class="relative flex-1 max-w-sm">
                    <Search
                        size={16}
                        class="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted"
                    />
                    <input
                        type="text"
                        placeholder="Search files..."
                        bind:value={search}
                        class="w-full pl-9 pr-3 py-2 rounded-lg border border-border bg-surface-alt text-text
                            placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-accent/40 focus:border-accent text-sm"
                    />
                </div>
                {#if filtered.length > 0}
                    <button
                        onclick={selectAll}
                        class="text-sm text-text-muted hover:text-text transition-colors"
                    >
                        {filtered.every((i) => selected.has(i.path))
                            ? "Deselect all"
                            : "Select all"}
                    </button>
                {/if}
            </div>

            {#if loading}
                <div class="space-y-2">
                    {#each Array(6) as _}
                        <div
                            class="h-12 rounded-lg bg-surface-alt border border-border animate-pulse"
                        ></div>
                    {/each}
                </div>
            {:else if filtered.length === 0}
                <div class="text-center py-16 text-text-muted">
                    <FolderPlus size={40} class="mx-auto mb-3 opacity-40" />
                    <p>{search ? "No matching files" : "This directory is empty"}</p>
                </div>
            {:else}
                <div class="space-y-1">
                    {#each filtered as item (item.path)}
                        <FileRow
                            {item}
                            selected={selected.has(item.path)}
                            onselect={toggleSelect}
                            onnavigate={browse}
                        />
                    {/each}
                </div>
            {/if}
        </div>

        <!-- staged files panel (right) -->
        {#if selectedCount > 0}
            <div class="w-72 shrink-0 hidden lg:block">
                <div class="sticky top-8 rounded-xl border border-border bg-surface-alt p-4">
                    <div class="flex items-center justify-between mb-3">
                        <h3 class="text-sm font-semibold text-text">
                            Staged ({selectedCount})
                        </h3>
                        <button
                            onclick={() => (selected = new Set())}
                            class="text-xs text-text-muted hover:text-danger transition-colors"
                        >
                            Clear all
                        </button>
                    </div>
                    <div class="space-y-1 max-h-[60vh] overflow-y-auto">
                        {#each [...selected] as path (path)}
                            <div
                                class="flex items-center justify-between gap-2 px-2 py-1.5 rounded-lg text-sm
                                    text-text hover:bg-surface-hover group"
                            >
                                <span class="truncate" title={path}>{filename(path)}</span>
                                <button
                                    onclick={() => removeSelected(path)}
                                    class="shrink-0 p-0.5 rounded text-text-muted opacity-0 group-hover:opacity-100
                                        hover:text-danger transition-all"
                                    title="Remove"
                                >
                                    <X size={14} />
                                </button>
                            </div>
                        {/each}
                    </div>
                </div>
            </div>
        {/if}
    </div>
</div>

<Modal bind:open={generateOpen} title="Generate Download Link">
    <form
        onsubmit={(e) => {
            e.preventDefault();
            handleGenerate();
        }}
        class="space-y-4"
    >
        <!-- staged files -->
        <div>
            <button
                type="button"
                onclick={() => (stagedExpanded = !stagedExpanded)}
                class="flex items-center gap-1 text-sm font-medium text-text mb-1"
            >
                <span class="text-text-muted">{stagedExpanded ? "▾" : "▸"}</span>
                Selected Files ({selectedCount})
            </button>
            {#if stagedExpanded}
                <div class="max-h-40 overflow-y-auto rounded-lg border border-border bg-surface-alt">
                    {#each [...selected] as path (path)}
                        <div class="flex items-center justify-between px-3 py-1.5 text-sm text-text hover:bg-surface-hover">
                            <span class="truncate" title={path}>{filename(path)}</span>
                            <button
                                type="button"
                                onclick={() => removeSelected(path)}
                                class="shrink-0 p-0.5 rounded text-text-muted hover:text-danger transition-colors"
                                title="Remove from selection"
                            >
                                <X size={14} />
                            </button>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>

        <div>
            <label
                for="gen-name"
                class="block text-sm font-medium text-text mb-1"
                >Name (optional)</label
            >
            <input
                id="gen-name"
                type="text"
                bind:value={genName}
                placeholder="Custom download name"
                class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
					placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-accent/40 text-sm"
            />
        </div>

        <div>
            <label
                for="gen-max"
                class="block text-sm font-medium text-text mb-1"
                >Max Downloads</label
            >
            <input
                id="gen-max"
                type="number"
                min="1"
                bind:value={genMaxDownloads}
                placeholder="Unlimited"
                class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
					placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-accent/40 text-sm"
            />
        </div>

        <!-- expiry -->
        <div>
            <span class="block text-sm font-medium text-text mb-1">Expires</span>
            <div class="flex items-center gap-2">
                <label class="flex items-center gap-2 text-sm text-text-muted">
                    <input
                        type="checkbox"
                        bind:checked={genNoExpiry}
                        class="rounded border-border"
                    />
                    Never
                </label>
            </div>
            {#if !genNoExpiry}
                <div class="flex items-center gap-2 mt-2">
                    <input
                        type="number"
                        min="1"
                        bind:value={genExpiryAmount}
                        class="w-20 px-3 py-2 rounded-lg border border-border bg-surface text-text
                            focus:outline-none focus:ring-2 focus:ring-accent/40 text-sm"
                    />
                    <select
                        bind:value={genExpiryUnit}
                        class="flex-1 px-3 py-2 rounded-lg border border-border bg-surface text-text
                            focus:outline-none focus:ring-2 focus:ring-accent/40 text-sm"
                    >
                        {#each Object.keys(EXPIRY_UNITS) as unit}
                            <option value={EXPIRY_UNITS[unit]}>{unit}</option>
                        {/each}
                    </select>
                </div>
            {/if}
        </div>

        <!-- archive format (multi-file only) -->
        {#if selectedCount > 1}
            <div>
                <label
                    for="gen-format"
                    class="block text-sm font-medium text-text mb-1"
                    >Archive Format</label
                >
                <select
                    id="gen-format"
                    bind:value={genFormat}
                    class="w-full px-3 py-2 rounded-lg border border-border bg-surface text-text
                        focus:outline-none focus:ring-2 focus:ring-accent/40 text-sm"
                >
                    {#each Object.keys(ARCHIVE_FORMATS) as fmt}
                        <option value={ARCHIVE_FORMATS[fmt]}>{formatLabels[ARCHIVE_FORMATS[fmt]] ?? fmt}</option>
                    {/each}
                </select>
            </div>
        {/if}

        <div class="flex justify-end gap-3 pt-2">
            <button
                type="button"
                onclick={() => (generateOpen = false)}
                class="px-4 py-2 text-sm rounded-lg border border-border text-text-muted hover:bg-surface-hover transition-colors"
            >
                Cancel
            </button>
            <button
                type="submit"
                disabled={generating || selectedCount === 0}
                class="px-4 py-2 text-sm rounded-lg bg-accent text-text-inverse hover:bg-accent-hover
					transition-colors disabled:opacity-50"
            >
                {generating ? "Creating..." : "Create Link"}
            </button>
        </div>
    </form>
</Modal>
