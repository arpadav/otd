<script lang="ts">
	// --------------------------------------------------
	// imports
	// --------------------------------------------------
	import { Heart, Code } from 'lucide-svelte';
</script>

<footer class="border-t border-border bg-surface-alt mt-auto">
	<div class="max-w-6xl mx-auto px-4 py-4">
		<div class="flex flex-col sm:flex-row items-center justify-between gap-2 text-xs text-text-muted">
			<p class="flex items-center gap-1">
				OTD &mdash; One-Time Downloads. Made with <Heart size={12} class="text-danger" /> by aav
			</p>
			<div class="flex items-center gap-4">
				<a
					href="https://github.com/arpadav/otd"
					target="_blank"
					rel="noopener"
					class="flex items-center gap-1.5 hover:text-text transition-colors"
				>
					<Code size={14} />
					Source
				</a>
			</div>
		</div>
	</div>
</footer>
