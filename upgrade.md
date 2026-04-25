# **DropBridge (fileshare) Codebase Analysis & Enhancement Plan**

This document outlines a comprehensive upgrade path for DropBridge. Based on the project's tech stack (Tauri 2.0, Svelte 5, Rust/Tokio, and SQLite), these enhancements address common edge cases in local P2P networking, optimize async performance, and modernize state management.  
To streamline your workflow, copy the prompts from the Gemini CLI Prompts section below and execute them sequentially in your CLI to iteratively implement these enhancements.

## **1\. Architectural Fixes & Optimizations (Rust / Tauri / Tokio)**

### **A. Non-Blocking Database Operations**

* **Current State:** Using rusqlite in WAL mode.  
* **Issue:** rusqlite is synchronous. If executed on the main Tokio runtime threads, it can block the async event loop during heavy I/O (e.g., logging many files).  
* **Fix:** Ensure all SQLite operations are wrapped in tokio::task::spawn\_blocking. Alternatively, optimize rusqlite with a dedicated connection pool (e.g., using deadpool-sqlite), which is often a more direct, lightweight, and idiomatic path for Tauri.

### **B. mDNS Graceful Shutdown & AP Isolation**

* **Current State:** mdns-sd used for zero-config discovery.  
* **Issue:** If the Tauri app closes unexpectedly or without explicit unregistration, zombie mDNS records can linger on the local network. Furthermore, some routers implement "AP Isolation", completely blocking mDNS.  
* **Fix:** 1\. Hook into Tauri's RunEvent::Exit and WindowEvent::Destroyed to ensure a graceful mdns.unregister() call.  
  2\. Implement a **Manual IP Fallback**: A UI feature allowing users to input a target IP directly or scan a QR code if mDNS discovery fails.

### **C. Adaptive TCP Buffering & Backpressure**

* **Current State:** 1MB optimized fixed buffers for TCP framing.  
* **Issue:** While 1MB buffers are great for large files on gigabit networks, they cause high memory overhead if there are dozens of concurrent connections transferring smaller files.  
* **Fix:** Implement adaptive buffer sizing. Use tokio::io::copy\_buf for optimized chunked streaming with built-in backpressure, rather than manually allocating static 1MB blocks in memory.

## **2\. Security Hardening**

### **A. Per-IP Rate Limiting**

* **Current State:** Concurrent connection throttling via semaphores.  
* **Issue:** A global semaphore protects against socket exhaustion but allows a single malicious local actor (or a stuck loop on a peer device) to consume all available semaphore slots, effectively DoS-ing the app.  
* **Fix:** Implement a per-IP rate limiter, utilizing a caching crate like moka or a concurrent map like DashMap to count active connections per IP, to ensure fair resource distribution.

### **B. Strict Filename Sanitization**

* **Current State:** Filename sanitization against path traversal (../).  
* **Issue:** Needs to handle cross-platform constraints (e.g., Windows forbidden characters \< \> : " / \\ | ? \*) and Null-byte (\\0) injections.  
* **Fix:** Use a dedicated Rust sanitization crate (like sanitize-filename) to strip all OS-level invalid characters and prevent local file system corruption.

## **3\. UI/UX & Frontend Enhancements (Svelte 5\)**

### **A. Full Runes Migration**

* **Current State:** Svelte 5 frontend.  
* **Enhancement:** Ensure all global state (Network Peers, Active Transfers, Progress) is managed using Svelte 5 Runes ($state.raw for large data arrays, $derived for computed ETA/speeds) rather than legacy Svelte stores. This drastically reduces reactivity overhead.

### **B. Virtualized History List**

* **Current State:** Paginated SQLite history.  
* **Enhancement:** For high-volume users, rendering large paginated DOM lists can cause jank. Implement a "Virtual Scroller" for the history view, rendering only the DOM nodes currently visible in the viewport.

### **C. Folder/Directory Transfer Support**

* **Enhancement:** Instead of selecting individual files, allow users to drag-and-drop entire folders.  
* **Implementation:** The Rust backend should traverse the directory, pack it into an uncompressed tar stream on the fly, and send it as a single TCP stream, ensuring that the total byte size of the directory is pre-calculated so the frontend progress bar maintains accurate ETA and speed metrics. The receiver extracts the tar stream directly to the disk without keeping it in memory.

## **4\. Gemini CLI Implementation Prompts**

*Instructions: Run these prompts one by one in your Gemini CLI while pointing to the repository context. Tip: Review and test the generated code after each prompt before moving to the next one to ensure the project remains stable and regressions are caught early.*

### **Prompt 1: Graceful Shutdown & mDNS Fix**

"Analyze src-tauri/src/main.rs and the networking module. Update the Tauri builder to intercept the RunEvent::Exit lifecycle event. Ensure that the mDNS broadcaster is gracefully shut down and its thread aborted so no ghost records are left on the local network."

### **Prompt 2: Async SQLite Database wrapper**

"Review the current rusqlite database implementation. Refactor the database service so that all read/write operations (especially logging to the history table) are wrapped in tokio::task::spawn\_blocking to prevent blocking the async Tokio thread. Ensure the SQLite connection pooling is thread-safe."

### **Prompt 3: Per-IP Connection Limiting**

"Examine the TCP listener loop in the Rust backend. Upgrade the global semaphore rate limiting to a Per-IP rate limiting system. Use a thread-safe map (DashMap or std::collections::HashMap wrapped in a Mutex) to limit concurrent active connections from a single IP address to a maximum of 3."

### **Prompt 4: Robust Path Sanitization**

"Inspect the file receiving logic in the Rust backend. Improve the filename sanitization. Ensure it strictly strips Windows-forbidden characters (\<, \>, :, ", /, \\, |, ?, \*), null bytes, and control characters before generating the local save path, preventing both path traversal and file-system errors."

### **Prompt 5: Svelte 5 Runes State Migration**

"Review the Svelte 5 frontend components handling Active Transfers and Peer Discovery. Refactor the state management to strictly use Svelte 5 $state and $derived runes. For arrays holding large amounts of transfer data or historical logs, use $state.raw to optimize reactivity performance."

### **Prompt 6: Manual IP Fallback UI**

"Add a new 'Direct Connect' feature to the Svelte frontend. Create a small UI component that allows a user to input an IP address and port manually. Wire this up to a new Tauri command in the Rust backend that attempts a direct TCP handshake with the provided IP, bypassing mDNS discovery."