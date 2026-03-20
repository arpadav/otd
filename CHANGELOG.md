# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

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
- Dashboard statistics endpoint (active/used/expired tokens, total downloads, uptime)
- About page and login/logout flow
