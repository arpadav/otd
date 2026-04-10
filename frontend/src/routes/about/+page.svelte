<script lang="ts">
    // --------------------------------------------------
    // imports
    // --------------------------------------------------
    import { Code, Globe, Coffee } from "lucide-svelte";
</script>

<svelte:head>
    <title>About - OTD</title>
</svelte:head>

<div class="max-w-2xl mx-auto px-4 py-8 space-y-8">
    <div>
        <h1 class="text-2xl font-bold text-text">About OTD</h1>
    </div>

    <div class="card space-y-6">
        <div class="space-y-4 text-sm leading-relaxed text-text/80">
            <p>
                <strong class="text-text">OTD</strong> (one-time downloads) is a
                self-hosted, lightweight tool for creating
                <strong class="text-text">secure download links</strong>
            </p>
            <p>No accounts, no cloud storage, no tracking.</p>
            <p>
                Built with <strong class="text-text">Rust</strong> for performance
                and reliability.
            </p>
            <p>
                Fully free and open source. OTD is <strong class="text-text"
                    >MIT Licensed</strong
                >.
            </p>
        </div>

        <hr class="border-border" />

        <div class="space-y-3">
            <h2 class="text-sm font-semibold text-text uppercase tracking-wide">
                Links
            </h2>
            <ul class="space-y-2 text-sm">
                <li>
                    <a
                        href="https://github.com/arpadav/otd"
                        target="_blank"
                        rel="noopener"
                        class="inline-flex items-center gap-2 text-accent hover:text-accent-hover transition-colors"
                    >
                        <Code size={16} />
                        GitHub
                    </a>
                </li>
                <li>
                    <a
                        href="https://arpadvoros.com"
                        target="_blank"
                        rel="noopener"
                        class="inline-flex items-center gap-2 text-accent hover:text-accent-hover transition-colors"
                    >
                        <Globe size={16} />
                        arpadvoros.com
                    </a>
                </li>
            </ul>
        </div>

        <hr class="border-border" />

        <div class="space-y-3">
            <h2 class="text-sm font-semibold text-text uppercase tracking-wide">
                Support
            </h2>
            <p class="text-sm text-text-muted">
                If OTD is useful to you, consider supporting the project.
            </p>
            <div class="flex flex-col sm:flex-row gap-3">
                <a
                    href="https://buymeacoffee.com/arpadav"
                    target="_blank"
                    rel="noopener"
                    class="inline-flex items-center justify-center gap-2 px-4 py-2 text-sm font-semibold
						text-yellow-900 bg-yellow-400 rounded-lg hover:bg-yellow-300 transition-colors"
                >
                    <Coffee size={16} />
                    Buy Me a Coffee
                </a>
                <div
                    class="inline-flex items-center gap-2 px-4 py-2 text-sm text-text-muted
					bg-surface-alt border border-border rounded-lg"
                >
                    <!-- <Bitcoin size={16} /> -->
                    <span>&#8383;</span>
                    <code class="font-mono text-xs">bc1q...</code>
                </div>
            </div>
        </div>
    </div>
</div>
