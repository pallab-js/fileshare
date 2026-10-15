# DropBridge

[![CI](https://github.com/pallab-js/fileshare/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/fileshare/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)

Cross-platform peer-to-peer file sharing over your local network. Discover devices automatically, send files directly — no cloud and no account required.

## Features

- **Automatic discovery** — finds other DropBridge devices on the same LAN via mDNS.
- **Direct connect** — fall back to manual IP + port when mDNS is blocked (e.g. AP isolation).
- **Streaming transfers** — length-prefixed TCP framing with 1 MB buffers and live progress, speed, and ETA.
- **Transfer history** — stored locally in SQLite (WAL mode), filterable and paginated.
- **Desktop native** — native file pickers, system notifications, and graceful mDNS cleanup on exit.
- **Safe writes** — incoming filenames are sanitized and collisions are resolved automatically (`file (1).png`).
- **Basic DoS protection** — concurrent inbound connections are capped at 3 per source IP.

## How it works

```
Device A                                 Device B
  │  1. mDNS discovery (_dropbridge._tcp)  │
  │ ◄────────────────────────────────────►│
  │  2. TCP connect + JSON transfer request│
  │ ──────────────────────────────────────►│
  │  3. accept / decline response          │
  │ ◄──────────────────────────────────────│
  │  4. streamed file bytes + progress     │
  │ ──────────────────────────────────────►│
```

Each transfer opens a dedicated TCP connection. A length-prefixed JSON header describes the file,
the receiver accepts or declines, and the payload is then streamed with periodic progress events
emitted to the UI.

## Tech stack

| Layer     | Technology                                               |
| --------- | -------------------------------------------------------- |
| Frontend  | Svelte 5 (Runes), TypeScript, Tailwind CSS               |
| Backend   | Rust, Tauri 2, Tokio                                     |
| Storage   | SQLite via `rusqlite` (off-thread with `spawn_blocking`) |
| Discovery | mDNS via `mdns-sd`                                       |

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/) (`npm install -g pnpm`)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Platform build dependencies from the [Tauri prerequisites](https://tauri.app/start/prerequisites/)

### Install and run

```bash
git clone https://github.com/pallab-js/fileshare.git
cd fileshare
pnpm install
pnpm tauri dev
```

### Build a release bundle

```bash
pnpm tauri build
```

The installer for your current platform is written to `src-tauri/target/release/bundle/`.

## Security & privacy

DropBridge is designed for use on **trusted local networks**:

- Discovery and transfers never leave your LAN — there are no external relay or handshake servers.
- Peer metadata (name, IP, last seen) is stored only in a local SQLite database.

> [!IMPORTANT]
> Transfers are currently sent **unencrypted** over plain TCP, and peers are not
> authenticated. Anyone on the same network can send you a transfer request (which
> you must accept) and could observe traffic in transit. Do not use DropBridge on
> untrusted networks for sensitive data. Transport encryption and peer
> authentication are planned — see the [Roadmap](#roadmap).

## Roadmap

- [ ] Transport encryption (TLS) for file streams
- [ ] Peer authentication / pairing
- [ ] Transfer integrity verification (checksums)
- [ ] Resumable transfers

## Project layout

```
src/            SvelteKit frontend (routes, stores, types, utils)
src-tauri/src/
  main.rs       Tauri commands and app wiring
  mdns.rs       Service registration & peer discovery
  transfer.rs   TCP transfer protocol & streaming
  db.rs         SQLite schema & queries
```

## Contributing

Contributions are welcome. Before opening a PR, make sure checks pass:

```bash
pnpm check
cd src-tauri && cargo check && cargo clippy --all-targets -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [DESIGN.md](DESIGN.md) for workflow and UI guidelines.

## License

Released under the [MIT License](LICENSE).
