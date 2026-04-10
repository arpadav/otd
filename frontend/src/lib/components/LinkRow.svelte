<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import {
        Copy,
        Trash2,
        RefreshCw,
        ExternalLink,
        Pencil,
        X,
        Check,
    } from "lucide-svelte";
    import type { TokenListItem, UpdateLinkRequest } from "$lib/types";
    import {
        LINK_STATUSES,
        EXPIRY_UNITS,
        EXPIRY_MULTIPLIERS,
    } from "$lib/constants";
    import StatusBadge from "./StatusBadge.svelte";
    import { addToast } from "$lib/stores/toast.svelte";

    // --------------------------------------------------
    // props
    // --------------------------------------------------
    /**
     * @prop link - the link/token entry to render, including status, download counts, and URLs
     * @prop ondelete - callback invoked with the token string when the delete button is clicked
     * @prop onrevive - callback invoked with the token string when the retry button is clicked on a failed link
     */
    /**
     * @prop link - the link/token entry to render, including status, download counts, and URLs
     * @prop ondelete - callback invoked with the token string when the delete button is clicked
     * @prop onrevive - callback invoked with the token string when the retry button is clicked on a failed link
     * @prop onedit - callback invoked with the token and update payload when the edit form is submitted
     */
    let {
        link,
        ondelete,
        onrevive,
        onedit,
    }: {
        link: TokenListItem;
        ondelete: (token: string) => void;
        onrevive: (token: string) => void;
        onedit: (token: string, req: UpdateLinkRequest) => void;
    } = $props();

    // --------------------------------------------------
    // edit state
    // --------------------------------------------------
    /** whether the inline edit form is visible */
    let editing = $state(false);
    /** edited max downloads value */
    let editMaxDownloads = $state(1);
    /** whether the edited link should have an expiry */
    let editHasExpiry = $state(false);
    /** edited expiry numeric value */
    let editExpiryValue = $state(1);
    /** edited expiry unit (e.g. "hours", "days") */
    let editExpiryUnit = $state(EXPIRY_UNITS.hours);
    /** true while the edit request is in flight */
    let saving = $state(false);

    // --------------------------------------------------
    // derived
    // --------------------------------------------------
    /** derives a single badge-compatible status string from the link's
     * archive state, expiry flag, and remaining download count */
    let status = $derived.by(() => {
        if (link.link_status === LINK_STATUSES.preparing)
            return LINK_STATUSES.preparing;
        if (link.link_status === LINK_STATUSES.failed)
            return LINK_STATUSES.failed;
        if (link.expired) return LINK_STATUSES.expired;
        if (link.remaining_downloads <= 0) return LINK_STATUSES.used;
        return LINK_STATUSES.active;
    });

    // --------------------------------------------------
    // edit functions
    // --------------------------------------------------
    /** expiry units ordered largest-first for decomposition */
    const unitsBySize = Object.entries(EXPIRY_MULTIPLIERS).sort(
        ([, a], [, b]) => b - a,
    );

    /**
     * Opens the inline edit form, pre-filling fields from the current link state
     *
     * Picks the largest expiry unit that divides evenly into the remaining
     * seconds, falling back to the smallest unit for odd remainders
     */
    function startEdit() {
        editMaxDownloads = link.max_downloads;
        editHasExpiry = link.expires_in_seconds !== null;
        // --------------------------------------------------
        // decompose remaining seconds into the best-fit unit
        // --------------------------------------------------
        if (link.expires_in_seconds !== null) {
            const secs = link.expires_in_seconds;
            const fit = unitsBySize.find(
                ([, mult]) => secs >= mult && secs % mult === 0,
            );
            if (fit) {
                editExpiryUnit = fit[0];
                editExpiryValue = secs / fit[1];
            } else {
                // --------------------------------------------------
                // no clean division - approximate with minutes
                // --------------------------------------------------
                editExpiryUnit = EXPIRY_UNITS.minutes;
                editExpiryValue = Math.max(
                    1,
                    Math.round(secs / EXPIRY_MULTIPLIERS[EXPIRY_UNITS.minutes]),
                );
            }
        } else {
            editExpiryValue = 1;
            editExpiryUnit = EXPIRY_UNITS.hours;
        }
        editing = true;
    }

    /**
     * Submits the edit form by computing `expires_in_seconds` from the chosen
     * unit and value, then delegates to the parent's `onedit` callback
     */
    async function submitEdit() {
        saving = true;
        // --------------------------------------------------
        // convert unit + value back to total seconds
        // --------------------------------------------------
        const expires_in_seconds = editHasExpiry
            ? Math.max(1, editExpiryValue) *
              (EXPIRY_MULTIPLIERS[editExpiryUnit] ??
                  EXPIRY_MULTIPLIERS[EXPIRY_UNITS.hours])
            : null;
        onedit(link.token, {
            max_downloads: Math.max(1, editMaxDownloads),
            expires_in_seconds,
        });
        editing = false;
        saving = false;
    }

    // --------------------------------------------------
    // functions
    // --------------------------------------------------
    /**
     * Formats a remaining-seconds value into a compact human-readable expiry string
     *
     * Returns 'No expiry' for null, 'Expired' for non-positive values, and
     * a short duration string (e.g. '45m', '3h', '2d') for positive values
     *
     * @param seconds - remaining seconds until expiry, or null for no expiry
     */
    function formatExpiry(seconds: number | null): string {
        // --------------------------------------------------
        // handle null (permanent) and already-expired cases
        // --------------------------------------------------
        if (seconds === null) return "No expiry";
        if (seconds <= 0) return "Expired";
        // --------------------------------------------------
        // scale to the most readable unit: minutes, hours, days
        // --------------------------------------------------
        if (seconds < 3600) return `${Math.ceil(seconds / 60)}m`;
        if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
        return `${Math.floor(seconds / 86400)}d`;
    }

    /**
     * Copies the download URL to the system clipboard
     *
     * Uses the Clipboard API to write the link's download URL,
     * then shows a brief success toast for user feedback
     */
    async function copyUrl() {
        // --------------------------------------------------
        // write the download URL to the system clipboard
        // --------------------------------------------------
        await navigator.clipboard.writeText(link.download_url);
        // --------------------------------------------------
        // show a brief success toast (auto-dismisses in 2s)
        // --------------------------------------------------
        addToast("Link copied to clipboard", "success", 2000);
    }
</script>

<div
    class="flex items-center gap-4 px-4 py-3 rounded-lg border border-border hover:bg-surface-hover transition-colors"
>
    <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 mb-1">
            <span class="text-sm font-medium text-text truncate"
                >{link.name}</span
            >
            <StatusBadge {status} />
        </div>
        <div class="flex items-center gap-3 text-xs text-text-muted">
            <span>{link.download_count}/{link.max_downloads} downloads</span>
            <span>{formatExpiry(link.expires_in_seconds)}</span>
            {#if link.paths.length > 1}
                <span>{link.paths.length} files</span>
            {/if}
        </div>
    </div>
    <div class="flex items-center gap-1 shrink-0">
        {#if status === LINK_STATUSES.active}
            <button
                onclick={copyUrl}
                title="Copy download URL"
                class="p-2 rounded-lg text-text-muted hover:text-text hover:bg-surface-active transition-colors"
            >
                <Copy size={16} />
            </button>
            <a
                href={link.download_url}
                target="_blank"
                rel="noopener"
                title="Open download URL"
                class="p-2 rounded-lg text-text-muted hover:text-text hover:bg-surface-active transition-colors"
            >
                <ExternalLink size={16} />
            </a>
        {/if}
        {#if status === LINK_STATUSES.failed}
            <button
                onclick={() => onrevive(link.token)}
                title="Retry archive"
                class="p-2 rounded-lg text-warning hover:bg-warning/10 transition-colors"
            >
                <RefreshCw size={16} />
            </button>
        {/if}
        <button
            onclick={editing ? () => (editing = false) : startEdit}
            title={editing ? "Cancel edit" : "Edit link"}
            class="p-2 rounded-lg text-text-muted hover:text-text hover:bg-surface-active transition-colors"
        >
            {#if editing}
                <X size={16} />
            {:else}
                <Pencil size={16} />
            {/if}
        </button>
        <button
            onclick={() => ondelete(link.token)}
            title="Delete link"
            class="p-2 rounded-lg text-text-muted hover:text-danger hover:bg-danger/10 transition-colors"
        >
            <Trash2 size={16} />
        </button>
    </div>
</div>

{#if editing}
    <form
        onsubmit={(e) => {
            e.preventDefault();
            submitEdit();
        }}
        class="flex items-center gap-3 px-4 py-2 -mt-1 rounded-b-lg border border-t-0 border-border bg-surface-alt"
    >
        <label class="flex items-center gap-1.5 text-xs text-text-muted">
            Max downloads
            <input
                type="number"
                min="1"
                bind:value={editMaxDownloads}
                class="w-16 px-2 py-1 rounded border border-border bg-surface text-text text-xs
                    focus:outline-none focus:ring-2 focus:ring-accent"
            />
        </label>

        <label class="flex items-center gap-1.5 text-xs text-text-muted">
            <input
                type="checkbox"
                bind:checked={editHasExpiry}
                class="accent-accent"
            />
            Expiry
        </label>

        {#if editHasExpiry}
            <div class="flex items-center gap-1">
                <input
                    type="number"
                    min="1"
                    bind:value={editExpiryValue}
                    class="w-16 px-2 py-1 rounded border border-border bg-surface text-text text-xs
                        focus:outline-none focus:ring-2 focus:ring-accent"
                />
                <select
                    bind:value={editExpiryUnit}
                    class="px-2 py-1 rounded border border-border bg-surface text-text text-xs
                        focus:outline-none focus:ring-2 focus:ring-accent"
                >
                    {#each Object.keys(EXPIRY_UNITS) as unit}
                        <option value={EXPIRY_UNITS[unit]}>{unit}</option>
                    {/each}
                </select>
            </div>
        {/if}

        <button
            type="submit"
            disabled={saving}
            title="Save changes"
            class="ml-auto p-1.5 rounded-lg bg-accent text-white hover:bg-accent-hover transition-colors
                disabled:opacity-50"
        >
            <Check size={14} />
        </button>
    </form>
{/if}
