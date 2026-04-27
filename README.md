# otd

One-time download link server.

Run otd on the machine that holds your files and generate expiring, limited-use download links to share with others. No third-party upload, no account required on the recipient side. Send a colleague a link to a 5GB folder; the link self-destructs after N downloads or X time.

## UI

<img width="1042" height="414" alt="frontpage" src="https://github.com/user-attachments/assets/ab7ee872-101e-4d89-9234-95326d0ce97e" />
<img width="1041" height="569" alt="links" src="https://github.com/user-attachments/assets/4880eed5-fd83-42d7-a54d-96a5ff9cae22" />

## Quick Start

TODO: add docker and truenas here

The admin interface is at `http://127.0.0.1:15204` and the download server listens on `http://0.0.0.0:15205`. No configuration file needed - defaults work out of the box.

## Installation

TODO: add docker and truenas here

### From source

```sh
git clone https://github.com/arpadav/otd.git
cd otd
cargo build --release
./target/release/otd
```

## Configuration

Configuration is split into two categories: **CLI flags** (ephemeral, set at startup) and **persistent settings** (saved to disk, changed at runtime via the admin UI or API).

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--admin-host` | `127.0.0.1` | Bind address for the admin interface. Loopback by default to prevent external access |
| `--admin-port` | `15204` | Port for the admin interface and API |
| `--download-host` | `0.0.0.0` | Bind address for the download server. Binds all interfaces so recipients can reach it |
| `--download-port` | `15205` | Port for serving file downloads |
| `--base-path` | Current directory | Root directory for the file browser. All paths are sandboxed to this directory |
| `--log-level` | `info` | Minimum log level: `trace`, `debug`, `info`, `warn`, `error`. Overridden by `OTD_LOG` |
| `--log-file` | *(none)* | Write logs to this file in addition to stdout. Overridden by `OTD_LOG_FILE` |
| `--https` | *(disabled)* | Use `https://` in generated download URLs (not implemented; otd does not terminate TLS - put a reverse proxy in front) |

Run `otd --help` for the full list.

### Config Directory

All configs and links lives under the OTD data directory, determined by `$XDG_DATA_HOME/otd/` or falling back to `$HOME/.local/share/otd/`:

```
~/.local/share/otd/
  config.toml          # Persistent settings (password hash, download URL)
  links/               # Per-link JSON files for state persistence
    <token>.json
```

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `OTD_LOG` | Override `--log-level` (takes precedence over CLI) |
| `OTD_LOG_FILE` | Override `--log-file` (takes precedence over CLI) |

## API Reference

### Admin (default: `127.0.0.1:15204`)

- `GET /` - Web interface (SPA)
- `POST /api/auth/login` / `POST /api/auth/logout` - Session login and logout
- `GET /api/theme` / `PUT /api/theme` - Theme preference
- `GET /api/stats` - Dashboard statistics
- `GET /api/browse?path=<path>` - Browse files in a directory
- `GET /api/links` / `POST /api/links` - List or create download links
- `DELETE /api/links/{token}` / `POST /api/links/{token}/revive` - Delete or revive a link
- `DELETE /api/links?filter=<used|expired|all>` - Bulk delete links
- `GET /api/settings` / `PUT /api/settings` - Read or update persistent settings
- `POST /api/settings/password` - Change admin password

### Download (default: `0.0.0.0:15205`)

- `GET /<filename>?k=<token>` - Download the file or archive associated with the token

## License

MIT - see [LICENSE](LICENSE).
