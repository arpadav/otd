<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import Modal from './Modal.svelte';
	import { AlertTriangle } from 'lucide-svelte';

	// --------------------------------------------------
	// props
	// --------------------------------------------------
	/**
	 * @prop open - Controls dialog visibility (two-way bindable)
	 * @prop title - Heading text shown in the modal header
	 * @prop message - Body text explaining the action requiring confirmation
	 * @prop confirmLabel - Label for the destructive confirm button
	 * @prop onconfirm - Callback invoked when the user confirms the action
	 */
	let {
		open = $bindable(false),
		title = 'Confirm',
		message = 'Are you sure?',
		confirmLabel = 'Confirm',
		onconfirm,
	}: {
		open: boolean;
		title?: string;
		message?: string;
		confirmLabel?: string;
		onconfirm: () => void;
	} = $props();

	/**
	 * Closes the dialog and triggers the confirm callback.
	 *
	 * Closes the modal first to prevent double-clicks, then invokes the
	 * parent-supplied handler so the parent can perform the destructive action.
	 */
	function handleConfirm() {
		// --------------------------------------------------
		// close the dialog immediately to prevent re-triggering
		// --------------------------------------------------
		open = false;
		// --------------------------------------------------
		// invoke the parent's confirmation handler
		// --------------------------------------------------
		onconfirm();
	}
</script>

<Modal bind:open {title}>
	<div class="flex items-start gap-3 mb-6">
		<div class="text-warning mt-0.5">
			<AlertTriangle size={20} />
		</div>
		<p class="text-text-muted text-sm">{message}</p>
	</div>
	<div class="flex justify-end gap-3">
		<button
			onclick={() => (open = false)}
			class="px-4 py-2 text-sm rounded-lg border border-border text-text-muted hover:bg-surface-hover transition-colors"
		>
			Cancel
		</button>
		<button
			onclick={handleConfirm}
			class="px-4 py-2 text-sm rounded-lg bg-danger text-text-inverse hover:opacity-90 transition-opacity"
		>
			{confirmLabel}
		</button>
	</div>
</Modal>
