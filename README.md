# DropBridge

[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5.0-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

DropBridge is a professional-grade, cross-platform P2P file-sharing application built for speed, security, and simplicity. It allows you to discover devices on your local network and transfer files instantly without the need for cloud services or complex configurations.

![DropBridge Hero](https://via.placeholder.com/800x400.png?text=DropBridge+UI+Placeholder)

## ✨ Features

- **Instant Peer Discovery**: Automatic local network device discovery via mDNS.
- **High-Performance Transfers**: Custom TCP framing protocol optimized for large file transfers.
- **Professional Design**: Dark-mode native interface inspired by the Supabase design system.
- **Transfer Control**: Real-time speed tracking, ETA estimation, and transfer cancellation.
- **Secure by Default**: 4GB file size limits, consent timeouts, and strict Content Security Policies (CSP).
- **Persistent History**: Full transfer log stored in a local SQLite database with WAL mode enabled.
- **Cross-Platform**: Seamlessly works across macOS, Windows, and Linux.
- **Desktop Integration**: Native file pickers and system notifications.

## 🚀 Tech Stack

- **Frontend**: Svelte 5 (Runes), TypeScript, Tailwind CSS
- **Backend**: Rust, Tauri 2, Tokio (Async I/O)
- **Networking**: mDNS (Service Discovery), TCP (File Streaming)
- **Database**: SQLite (via `rusqlite`)

## 🛠️ Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install) (v1.75+)
- [pnpm](https://pnpm.io/installation)

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

3.  **Run in development mode**:
    ```bash
    pnpm tauri dev
    ```

### Building for Production

```bash
pnpm tauri build
```

## 🛡️ Security & Privacy

DropBridge is designed with a "Local First" philosophy:
- **No Cloud**: Your files never leave your local network.
- **Framing Protocol**: Uses length-prefixed TCP segments to prevent buffer overflow and injection attacks.
- **Consent Gate**: All incoming transfers require manual approval (unless configured otherwise in settings).
- **Sanitized Discovery**: Peer identities are sanitized to prevent mDNS spoofing.

## 🤝 Contributing

Contributions are welcome! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

Built with ❤️ by the DropBridge Team.
