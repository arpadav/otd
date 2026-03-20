# otd

One-time download link server.

Generate expiring, limited-use download links for files on your machine. Built with a dual-port architecture: an admin interface for browsing files and managing links, and a separate download server for serving files to recipients.

## Features

- **Dual-port architecture** — admin UI and download server run on separate ports for network isolation
- **One-time / N-time downloads** — configurable per-link download limits
- **Link expiration** — optional time-based expiry on generated links
- **Multi-file ZIP** — select multiple files and serve them as a single ZIP archive
- **File browser UI** — web-based file browser with search and multi-select
- **Bearer token auth** — protect the admin interface with an API token
- **Session auth** — password-based login for external access with HTTP-only cookies
- **Loopback bypass** — localhost access skips authentication entirely
- **HTTPS/TLS** — optional TLS termination with configurable cert/key paths
- **Docker support** — multi-stage Dockerfile and docker-compose included
- **Parallel directory traversal** — optional `rayon`/`jwalk`-based parallel file walking

## Quick Start

```sh
cargo run
```

On first run, a default `otd-config.toml` is generated in the current directory. The admin interface is available at `http://127.0.0.1:15204` and the download server listens on `http://0.0.0.0:15205`.

## Installation

### From source

```sh
git clone https://github.com/arpadav/otd.git
cd otd
cargo build --release
./target/release/otd
```

### Docker

```sh
docker build -t otd:latest .
docker run -p 15204:15204 -p 15205:15205 -v /path/to/files:/vault:ro otd:latest
```

Or with docker compose:

```sh
docker compose up -d
```

## Configuration

All settings are read from `otd-config.toml`. A default config file is generated on first run if none exists. The config file location can be overridden with the `OTD_CONFIG_FILE` environment variable.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `admin_port` | `u16` | `15204` | Port for the admin interface |
| `admin_host` | `String` | `"127.0.0.1"` | Bind address for admin interface |
| `download_port` | `u16` | `15205` | Port for the download server |
| `download_host` | `String` | `"0.0.0.0"` | Bind address for download server |
| `download_base_url` | `Option<String>` | `None` | Custom base URL for generated download links |
| `base_path` | `String` | Current directory | Root directory for file serving |
| `buffer_size` | `usize` | `8192` | Buffer size in bytes for HTTP request reading |
| `max_request_size` | `usize` | `10485760` | Maximum request body size in bytes (10 MB) |
| `enable_https` | `bool` | `false` | Enable HTTPS/TLS |
| `cert_path` | `Option<String>` | `None` | Path to TLS certificate file |
| `key_path` | `Option<String>` | `None` | Path to TLS private key file |
| `admin_token` | `Option<String>` | `None` | Bearer token for admin API authentication |
| `admin_password` | `Option<String>` | `None` | Password for web-based admin login |
| `log_level` | `Option<String>` | `None` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `log_file` | `Option<String>` | `None` | Path to log file (parent directory must exist) |

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `OTD_CONFIG_FILE` | Override config file path (default: `otd-config.toml` in current directory) |
| `OTD_BASE_PATH` | Override `base_path` from config |
| `OTD_LOG` | Override `log_level` from config |

Environment variables take precedence over config file values.

## API Reference

### Admin Interface (default: `127.0.0.1:15204`)

| Method | Path | Purpose | Auth |
|--------|------|---------|------|
| `GET` | `/` | Web interface | Yes |
| `GET` | `/about` | About page | Yes |
| `GET` | `/login` | Login form | No |
| `POST` | `/login` | Submit login | No |
| `GET` | `/logout` | Clear session | Session |
| `GET` | `/api/browse?path=<path>` | Browse files in directory | Yes |
| `GET` | `/api/stats` | Dashboard statistics | Yes |
| `GET` | `/api/tokens` | List active download tokens | Yes |
| `POST` | `/api/generate` | Generate download link | Yes |
| `POST` | `/api/tokens/bulk-delete` | Bulk delete tokens by filter | Yes |
| `DELETE` | `/api/tokens/<token>` | Delete specific token | Yes |

### Download Server (default: `0.0.0.0:15205`)

| Method | Path | Purpose | Auth |
|--------|------|---------|------|
| `GET` | `/<filename>?k=<token>` | Download file using token | Token only |

## Authentication

otd uses a layered authentication model:

1. **Bearer token** — If `admin_token` is set, all admin requests must include an `Authorization: Bearer <token>` header. This takes priority over other auth methods.

2. **Loopback bypass** — Requests from `127.0.0.1` or `::1` skip authentication entirely. This is the default mode since `admin_host` binds to `127.0.0.1`.

3. **Session login** — When accessed from a non-loopback address and `admin_password` is configured, users are redirected to `/login`. After entering the correct password, a session cookie is issued (valid for 24 hours, `HttpOnly`, `SameSite=Strict`).

If accessed externally without `admin_password` configured, a `403 Forbidden` is returned.

## Security

- **Path traversal protection** — All file paths are canonicalized and verified to remain within `base_path`. Requests containing `../` sequences are blocked after symlink resolution.
- **Symlink escape blocking** — Symlinks are resolved via `canonicalize()` before the containment check. Symlinks pointing outside `base_path` are rejected.
- **Loopback-only admin** — The admin interface binds to `127.0.0.1` by default, preventing external access unless explicitly configured.
- **Port isolation** — Admin and download interfaces run on separate ports, allowing firewall rules to restrict admin access independently.
- **Request size limits** — Configurable maximum request size (default 10 MB).
- **Secure cookies** — Session cookies use `HttpOnly` and `SameSite=Strict` flags.

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `parallel` | Yes | Enables parallel directory traversal via `rayon` and `jwalk` |
| `doc-tests` | No | Exposes private items for doc-test compilation via `visibility` |

To build without parallel traversal:

```sh
cargo build --release --no-default-features
```

## License

MIT — see [LICENSE](LICENSE).
