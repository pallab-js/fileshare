# DropBridge

[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.svelte.io/badge/Svelte-5.0-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-3ecf8e.svg)](https://opensource.org/licenses/MIT)

**DropBridge** is a professional-grade, cross-platform P2P file-sharing application designed for the modern desktop. Built with Rust and Svelte 5, it offers a "Local First" experience—allowing you to discover and transfer files across your LAN with zero configuration, zero cloud dependencies, and uncompromising security.

![DropBridge Hero](static/images/hero.png)

## ✨ Core Features

- **🚀 High-Performance Streaming**: Custom length-prefixed TCP framing protocol with 1MB optimized buffers for maximum throughput on Gigabit networks.
- **🔍 Hybrid Discovery**: 
  - **Zero-Config**: Automatic peer discovery via mDNS (Bonjour/Avahi).
  - **Direct Connect**: Manual IP/Port fallback for environments where mDNS is blocked by AP Isolation.
- **📊 Real-Time Analytics**: Precise speed tracking using a rolling 3-second sampler, live ETA calculations, and aggregate bandwidth monitoring.
- **🛡️ Security Hardened**: 
  - **Robust Sanitization**: Strips Windows-forbidden characters, control characters, and null bytes to prevent path traversal and filesystem errors.
  - **Per-IP Throttling**: Limits concurrent connections to 3 per IP address to prevent local resource exhaustion and DoS.
  - **Collision Prevention**: Automatic unique path resolution (e.g., `file (1).png`).
  - **Graceful Shutdown**: Intercepts OS signals to cleanly unregister services, preventing "ghost" mDNS records.
- **📜 Smart History**: Filterable, paginated transfer logs stored in a high-concurrency SQLite database (WAL mode).
- **⚙️ Desktop Native**:
  - Native file/directory pickers using Tauri's dialog system.
  - System-level notifications for transfer requests and completion.
  - Automatic cleanup of stale peer records older than 24 hours.

## 🚀 Tech Stack

- **Frontend**: [Svelte 5](https://svelte.dev/) (Runes), TypeScript, [Tailwind CSS](https://tailwindcss.com/)
- **Backend**: [Rust](https://www.rust-lang.org/), [Tauri 2.0](https://tauri.app/)
- **Async Runtime**: [Tokio](https://tokio.rs/)
- **Database**: SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) (Async-wrapped with `spawn_blocking`)
- **Networking**: [mdns-sd](https://github.com/pro_logic/mdns-sd) for discovery
- **Design System**: Inspired by Supabase—Dark mode native, HSL color tokens, and geometric typography.

## 🛠️ Getting Started

### Prerequisites

- **Node.js**: v18 or later
- **Rust**: v1.75 or later (stable)
- **pnpm**: `npm install -g pnpm`

### Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/pallab-js/fileshare.git
    cd fileshare
    ```

2.  **Install dependencies**:
    ```bash
    pnpm install
    ```

3.  **Run in development**:
    ```bash
    pnpm tauri dev
    ```

### Production Build

To generate a production-ready installer for your current platform:
```bash
pnpm tauri build
```

## 🛡️ Security & Privacy Philosophy

DropBridge is built on the principle of **Network Sovereignty**:
- **Metadata Privacy**: Peer discovery is limited to your local network. No external "handshake" servers are used.
- **End-to-End Local**: File data flows directly from the sender's memory/disk to the receiver's.
- **Non-Blocking Safety**: All database and I/O operations are offloaded to background threads to ensure UI responsiveness even during heavy transfers.

## 🤝 Contributing

We welcome contributions from the community! 

1.  Check the [CONTRIBUTING.md](CONTRIBUTING.md) for our development workflow.
2.  Follow the [DESIGN.md](DESIGN.md) for UI guidelines (Supabase-inspired design system).
3.  Ensure `pnpm check` and `cd src-tauri && cargo check` pass before submitting PRs.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

Built with precision by the **DropBridge Team**.
