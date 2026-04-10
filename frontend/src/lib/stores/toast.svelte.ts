// --------------------------------------------------
// types
// --------------------------------------------------
/** Severity level of a toast notification */
export type ToastType = 'success' | 'error' | 'info' | 'warning';

/** A single toast notification entry managed by the toast store */
export interface Toast {
	/** Unique auto-incremented identifier used for targeted removal */
	id: number;
	/** Human-readable message displayed inside the toast */
	message: string;
	/** Visual severity variant controlling icon and color */
	type: ToastType;
}

// --------------------------------------------------
// state
// --------------------------------------------------
/** Monotonically increasing counter used to assign unique IDs to each toast */
let nextId = 0;
/** Reactive array of active toast notifications; consumed by the Toast UI component */
export const toasts = $state<Toast[]>([]);

// --------------------------------------------------
// functions
// --------------------------------------------------
/**
 * Adds a new toast notification and schedules its automatic removal.
 *
 * Pushes a new `Toast` entry onto the reactive `toasts` array and registers
 * a `setTimeout` that calls `removeToast` after the given duration.  Svelte
 * will re-render any component that reads `toasts` as soon as the push lands.
 *
 * @param message - Text to display inside the toast
 * @param type - Severity variant; defaults to `'info'`
 * @param duration - Time in milliseconds before the toast auto-dismisses; defaults to 4000
 */
export function addToast(message: string, type: ToastType = 'info', duration = 4000): void {
	// --------------------------------------------------
	// assign a stable ID and push the toast into state
	// --------------------------------------------------
	const id = nextId++;
	toasts.push({ id, message, type });
	// --------------------------------------------------
	// schedule auto-removal after the configured duration
	// --------------------------------------------------
	setTimeout(() => removeToast(id), duration);
}

/**
 * Removes a toast notification by its unique ID.
 *
 * Finds the entry in the reactive `toasts` array via `findIndex` and splices
 * it out in-place.  The splice mutates the `$state` array, which triggers a
 * Svelte re-render on all consumers.  If no toast with the given ID exists
 * (e.g. it was already manually dismissed) the call is a no-op.
 *
 * @param id - The `Toast.id` of the entry to remove
 */
export function removeToast(id: number): void {
	// --------------------------------------------------
	// locate the toast by ID and splice it out if found
	// --------------------------------------------------
	const idx = toasts.findIndex((t) => t.id === id);
	if (idx !== -1) toasts.splice(idx, 1);
}
