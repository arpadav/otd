<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import { fly } from "svelte/transition";
    import { toasts, removeToast } from "$lib/stores/toast.svelte";
    import {
        CircleCheck,
        CircleX,
        Info,
        TriangleAlert,
        X,
    } from "lucide-svelte";

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /** Maps each toast type to its corresponding icon component */
    const icons = {
        success: CircleCheck,
        error: CircleX,
        info: Info,
        warning: TriangleAlert,
    };

    /** Maps each toast type to its Tailwind background and text color classes */
    const colors = {
        success: "bg-success text-text-inverse",
        error: "bg-danger text-text-inverse",
        info: "bg-info text-text-inverse",
        warning: "bg-warning text-text-inverse",
    };
</script>

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2 w-80">
    {#each toasts as toast (toast.id)}
        <div
            class="flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg {colors[
                toast.type
            ]}"
            transition:fly={{ x: 300, duration: 300 }}
        >
            {#if icons[toast.type]}
                {@const Icon = icons[toast.type]}
                <Icon size={18} />
            {/if}
            <span class="flex-1 text-sm">{toast.message}</span>
            <button
                onclick={() => removeToast(toast.id)}
                class="opacity-70 hover:opacity-100 transition-opacity"
            >
                <X size={14} />
            </button>
        </div>
    {/each}
</div>
