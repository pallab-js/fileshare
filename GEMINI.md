# GEMINI.md - DropBridge Project Context

## Project Overview
**DropBridge** is a cross-platform, local-network file-sharing application built with **Tauri**, **SvelteKit (Svelte 5)**, and **Rust**. It enables seamless peer-to-peer file transfers between devices on the same network without requiring a central server.

### Key Technologies
- **Frontend:** Svelte 5 (Runes mode), TypeScript, Tailwind CSS, Lucide Icons.
- **Backend:** Rust (Tauri), mDNS for discovery (`mdns-sd`), TCP for transfers (`tokio`), SQLite for persistence (`rusqlite`).
- **Design System:** Inspired by Supabase—a dark-mode-native aesthetic with emerald green accents, geometric typography (Circular), and HSL-based color tokens.

### Architecture
1.  **Peer Discovery:** Uses mDNS to advertise and browse for `_dropbridge._tcp.local.` services. Peer information is synchronized between Rust and Svelte stores.
2.  **File Transfer:** Custom TCP protocol.
    - Sender sends a JSON `TransferRequest`.
    - Receiver prompts for consent.
    - If accepted, raw bytes are streamed over TCP with progress updates emitted via Tauri events.
3.  **Persistence:** SQLite database (`dropbridge.db`) stored in the application's data directory, logging all transfer history and known peers.
4.  **Security:** Transfers are local-only; users must manually accept incoming files.

---

## Building and Running

### Prerequisites
- [Node.js](https://nodejs.org/) (pnpm recommended)
- [Rust](https://www.rust-lang.org/) (via rustup)
- Tauri dependencies (platform-specific: `build-essential`, `libwebkit2gtk`, etc.)

### Key Commands
- `pnpm install`: Install frontend dependencies.
- `pnpm tauri dev`: Run the application in development mode (starts both frontend and Rust backend).
- `pnpm tauri build`: Build the production-ready application bundle.
- `pnpm check`: Run Svelte and TypeScript type-checking.
- `pnpm build`: Build the frontend assets only.

---

## Development Conventions

### Rust (Backend)
- **State Management:** Uses Tauri's `manage` for global states (`DbState`, `DiscoveryState`, `TransferState`).
- **Async:** Leverages `tokio` for networking and `tauri::async_runtime` for non-blocking operations.
- **Modules:** 
    - `mdns.rs`: Service discovery logic.
    - `transfer.rs`: TCP listener and client logic.
    - `db.rs`: Database schema and logging.

### Svelte (Frontend)
- **Svelte 5:** Utilizes `$state`, `$props`, and `$derived` runes for reactive state management.
- **Stores:** Global application state (peers, transfers) is managed in `src/lib/stores.ts`.
- **Tauri Integration:** Uses `@tauri-apps/api/core` for `invoke` and `@tauri-apps/api/event` for real-time listener updates.
- **Styling:** Follows the Supabase design system as documented in `DESIGN.md`. All new components should adhere to the HSL-based color palette and typography rules defined there.

### File Locations
- **Frontend Routes:** `src/routes/`
- **Shared Lib:** `src/lib/`
- **Rust Source:** `src-tauri/src/`
- **Tauri Config:** `src-tauri/tauri.conf.json`
- **Design Docs:** `DESIGN.md`
