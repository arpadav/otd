<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { page } from '$app/state';
	import { AlertTriangle } from 'lucide-svelte';

	// `page.status` and `page.error` are provided by SvelteKit's error
	// boundary - no local state needed; this component is purely presentational
</script>

<div class="flex flex-col items-center justify-center min-h-[60vh] text-center px-4">
	<AlertTriangle size={48} class="text-warning mb-4" />
	<h1 class="text-4xl font-bold text-text mb-2">{page.status}</h1>
	<p class="text-text-muted">{page.error?.message ?? 'Something went wrong'}</p>
	<a href="/" class="mt-6 px-4 py-2 rounded-lg bg-accent text-text-inverse hover:bg-accent-hover transition-colors">
		Back to Dashboard
	</a>
</div>
