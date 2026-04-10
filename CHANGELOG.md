# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.0-rc-3] - Unreleased

### Changed

- **Frontend migration**: replaced Dioxus fullstack (Rust/WASM) with SvelteKit (TypeScript) SPA
- Frontend embedded in binary via `rust-embed` for single-binary deployment
- Dioxus server function macros replaced with plain Axum JSON API handlers
- Authentication switched from bearer token to blake3-signed HttpOnly cookie sessions
- 3-stage Dockerfile: Node.js (frontend) -> Rust (backend) -> Debian slim (runtime)
- Makefile rewritten for SvelteKit + Cargo build pipeline
- Bundle script updated for new binary output path

### Added

- Theme system: 6 earthy presets (forest, clay, ocean, stone, sage, dusk) with light/dark variants
- Theme persistence at `$XDG_DATA_HOME/otd/theme.json` via `GET/PUT /api/theme`
- System color scheme detection (`prefers-color-scheme`) as initial default
- Dark mode toggle per theme
- Live theme preview with swatches on settings page
- Svelte 5 runes-based reactive state management
- Tailwind CSS with runtime-switchable CSS custom properties (`--otd-*`)
- Toast notification system with auto-dismiss
- Modal and confirmation dialog components
- Responsive mobile navigation menu
- File browser search filtering
- Link generation modal with expiry presets (1h, 24h, 7d, never)
- Direct clipboard copy for download URLs
- Link status badges (active, used, expired, preparing, failed)
- Bulk delete actions for used and expired links
- Adaptive polling: 2s when archives preparing, 30s idle
- Dashboard quick-action cards
- About section consolidated into settings page
- Shared constants JSON between Rust and TypeScript for theme/status strings

### Removed

- Dioxus dependency and all WASM compilation targets
- `cfg`-gated feature attributes (`web`, `server`, `doc-tests` features)
- Client-side Rust components, pages, SVG modules, CSS classes module
- `dioxus-cli` build tooling
- Standalone about page (merged into settings)

## [0.0.0-rc-2] - Unreleased

### Added

- Adding Dioxus fullstack

### Removed

- Standalone JS / CSS and minimal HTML (custom HTML parsing) removed

## [0.0.0-rc-1] - Unreleased

### Added

- Dual-port HTTP server: admin interface and download server on separate ports
- Web-based file browser with directory traversal and search
- One-time and N-time download link generation with configurable limits
- Optional time-based link expiration
- Multi-file ZIP archive creation with Deflate compression
- Archive caching in `/tmp/otd-cache/` with content-hash-based cache keys
- Bearer token authentication for admin API
- Session-based authentication with password login for external access
- Loopback bypass for localhost access without credentials
- HTTPS/TLS support with configurable certificate and key paths
- TOML-based configuration with auto-generation of defaults on first run
- Environment variable overrides (`OTD_CONFIG_FILE`, `OTD_BASE_PATH`, `OTD_LOG`)
- Path traversal and symlink escape protection on all file operations
- Structured logging via `tracing` with configurable log level and optional file output
- Docker support with multi-stage build and docker-compose configuration
- Optional parallel directory traversal via `rayon` and `jwalk` (`parallel` feature)
- Token management: list, delete, bulk-delete (used/expired/all)
- Dashboard statistics endpoint (active/used/expired links, total downloads, uptime)
- About page and login/logout flow
