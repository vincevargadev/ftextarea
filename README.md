# ftextarea

A distraction-free, full-screen textarea for quick notes and temporary text storage. No login, no sync, no formatting — just a textarea.

**[Live Demo →](https://ftextarea.vincevarga.dev)**

## Why?

Sometimes you just need a place to paste a link or jot down a thought. Most apps require login, sync to the cloud, or format your text automatically. **ftextarea** does none of that — it's just a textarea.

Your text is saved to your browser's localStorage and syncs across tabs. It works offline after the first visit. Nothing leaves your browser.

> **Note:** Browsers may clear localStorage periodically. This is designed for temporary, throwaway text — not long-term storage.

## Features

- **Full-screen textarea** — no distractions, just text
- **Auto-save** — content saved to localStorage with 250ms debounce
- **Multi-tab sync** — changes sync across browser tabs via storage events
- **Offline support** — works without internet after first visit (Service Worker)
- **Dark/light mode** — respects your system preference
- **No account needed** — all data stays in your browser

## Technology Stack

This project is intentionally minimal:

- **HTML, CSS, JavaScript** — vanilla, no frameworks
- **Rust + WebAssembly** — DOM manipulation and localStorage logic
- **Service Worker** — offline support

The goal was to build a practical tool while learning Rust/WASM integration without frameworks like Yew or Dioxus.

## Setup Guide

### Prerequisites

- **Rust** — install from [rustup.rs](https://rustup.rs/)
- **wasm-pack** — install with `cargo install wasm-pack`
- **miniserve** (optional) — for local testing: `cargo install miniserve`

### Building

Build the WebAssembly package:

```bash
wasm-pack build --target web
```

This creates a `pkg/` directory with the compiled WASM module and JavaScript bindings.

### Running Locally

Serve the project with any static file server:

```bash
miniserve . --port 8088
```

Then open [http://localhost:8088](http://localhost:8088).

### Testing

Run Rust tests:

```bash
cargo test
```

## Deployment

This project deploys to a Scaleway server using Caddy as the web server.

### Quick Deploy

```bash
./release.sh
```

### Debug Deploy

For additional health checks and verbose output:

```bash
./release.sh --debug
```

### What the Release Script Does

1. Builds the WebAssembly package with `wasm-pack`
2. Backs up the existing deployment on the server
3. Copies the Caddy configuration to `/etc/caddy/conf.d/`
4. Deploys static files and WASM package to `/var/www/ftextarea.vincevarga.dev/`
5. Reloads Caddy
6. Cleans up old backups (keeps latest 5)

### Server Requirements

- Caddy installed and running as a systemd service
- SSH access configured (`~/.ssh/id_ed25519_scaleway`)

## Project Structure

```
ftextarea/
├── src/
│   └── lib.rs           # Rust/WASM logic
├── pkg/                  # Generated WASM package (after build)
├── index.html           # Main HTML file
├── style.css            # Styles (dark/light mode)
├── script.js            # WASM loader
├── sw.js                # Service Worker for offline support
├── Cargo.toml           # Rust dependencies
├── release.sh           # Deployment script
└── ftextarea.vincevarga.dev.caddy  # Caddy config
```

## License

MIT License — see [LICENSE](LICENSE) for details.

## Links

- **Live site:** [ftextarea.vincevarga.dev](https://ftextarea.vincevarga.dev)
- **Author:** [vincevarga.dev](https://vincevarga.dev)
