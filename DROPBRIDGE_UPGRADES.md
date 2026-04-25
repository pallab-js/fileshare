# DropBridge — Full Codebase Audit & Upgrade Plan

> **Project:** `pallab-js/fileshare` (DropBridge) — Tauri 2 + SvelteKit 5 + Rust  
> **Purpose of this doc:** Feed into Gemini CLI to implement all fixes, enhancements, and upgrades below in one pass.  
> **Reading order:** Critical Bugs → Security → Architecture → Feature Enhancements → Frontend Polish.

---

## CONTEXT SUMMARY

DropBridge is a local-network P2P file-sharing desktop app. Stack:
- **Frontend:** Svelte 5 (Runes), TypeScript, Tailwind CSS
- **Backend:** Rust, Tauri 2, tokio, mdns-sd, rusqlite
- **Protocol:** mDNS discovery + custom TCP transfer
- **DB:** SQLite via rusqlite (bundled)

---

## SECTION 1 — CRITICAL BUGS (implement first, they break functionality)

### 1.1 Dead `lib.rs` / `greet` command is never wired up

**File:** `src-tauri/src/lib.rs`  
**Problem:** The file still contains the scaffolded `greet` command and a `run()` entry point that is never called. `main.rs` defines its own `main()` independently. `lib.rs` is compiled as a library crate (`staticlib`, `cdylib`, `rlib`) but its `run()` is never invoked from `main.rs`.

**Fix:** Remove `lib.rs` entirely OR move all shared types/state into it and call `lib::run()` from `main.rs`. Recommended: delete `lib.rs`, keep all logic in `main.rs` and sibling modules. Remove `greet` from everywhere.

---

### 1.2 Hardcoded macOS-only download path

**File:** `src-tauri/src/transfer.rs`, line ~60  
**Problem:**
```rust
let download_dir = format!("/Users/{}/Downloads/DropBridge", whoami::username().unwrap_or_default());
```
This is macOS-only. Breaks on Windows and Linux.

**Fix:** Use Tauri's path resolver instead:
```rust
// In handle_incoming_transfer, pass app: AppHandle and use:
let download_dir = app.path().download_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join("DropBridge");
```
Requires `tauri::Manager` in scope. `download_dir()` resolves correctly on all platforms.

---

### 1.3 TCP request buffer overflow — large JSON silently truncated

**File:** `src-tauri/src/transfer.rs`  
**Problem:**
```rust
let mut buffer = [0; 1024];
let n = stream.read(&mut buffer).await ...;
let request = serde_json::from_slice::<TransferRequest>(&buffer[..n]);
```
`read()` is not guaranteed to read all bytes in one call. A `TransferRequest` with a long filename or sender name can exceed 1024 bytes or arrive in multiple TCP segments, causing silent parse failure.

**Fix:** Use a length-prefixed framing protocol:
```rust
// Sender side: write 4-byte LE length then JSON
let json = serde_json::to_vec(&request)?;
stream.write_all(&(json.len() as u32).to_le_bytes()).await?;
stream.write_all(&json).await?;

// Receiver side: read 4-byte length, then read exactly that many bytes
let mut len_buf = [0u8; 4];
stream.read_exact(&mut len_buf).await?;
let len = u32::from_le_bytes(len_buf) as usize;
let mut json_buf = vec![0u8; len];
stream.read_exact(&mut json_buf).await?;
let request: TransferRequest = serde_json::from_slice(&json_buf)?;
```
Apply the same pattern to the accept/decline response.

---

### 1.4 SHA256 hash computed but never verified — dead code

**File:** `src-tauri/src/transfer.rs`  
**Problem:** Both sender and receiver compute `Sha256` but the hash is never sent, stored, or compared. Integrity checking is completely non-functional.

**Fix (Option A — full integrity):**
- Sender: after streaming all bytes, send the 32-byte hash digest as a final fixed frame.
- Receiver: compare received hash to locally computed hash; emit `transfer_integrity_failed` event if mismatch, delete the partial file.

**Fix (Option B — remove dead code for now):**
Remove all `use sha2::{Sha256, Digest};` imports and hasher lines until the protocol supports it. Dead code in a transfer loop adds CPU cost for zero benefit.

---

### 1.5 `std::sync::Mutex` used in async context — potential deadlock

**File:** `src-tauri/src/transfer.rs` and `main.rs`  
**Problem:**
```rust
let mut pending = state.pending.lock().unwrap(); // std::sync::Mutex
```
Holding a `std::sync::Mutex` guard across `.await` points causes deadlocks and blocks the async executor thread.

**Fix:** Replace `Arc<Mutex<...>>` (std) with `Arc<tokio::sync::Mutex<...>>` for all state that is accessed from async functions. Update lock calls to `.lock().await`. The `DbState.conn` mutex in synchronous Tauri commands can remain `std::sync::Mutex` since commands run on a blocking thread pool, but `TransferState.pending` must be `tokio::sync::Mutex`.

---

### 1.6 History page: camelCase/snake_case field name mismatch

**File:** `src/routes/history/+page.svelte`  
**Problem:**
```ts
const mapped = history.map(h => ({
  fileName: h.fileName,  // WRONG — Rust serializes as file_name
  fileSize: h.fileSize,  // WRONG — serializes as file_size
  sender: h.peerName,    // WRONG — serializes as peer_name
}));
```
The `TransferHistory` struct in Rust uses `#[serde(rename_all = "camelCase")]`, so fields are correctly camelCased on the wire. **But** `h.peerName` is mapped to `sender` while the actual field to display the peer is `peerName` — this is only correct for received files, wrong for sent files where the receiver name isn't in the struct at all.

**Fix:**
```ts
const mapped = history.map(h => ({
  id: h.id,
  fileName: h.fileName,
  fileSize: h.fileSize,
  sender: h.direction === 'sent' ? 'You → ' + h.peerName : h.peerName,
  progress: 100,
  status: h.status,
  timestamp: h.timestamp,
}));
```
Also add `timestamp` rendering to the history table (currently missing).

---

### 1.7 `peer_removed` event emitted in Rust but not handled in frontend

**File:** `src-tauri/src/mdns.rs` emits `peer_removed`. `src/routes/+layout.svelte` has no listener for it.

**Fix:** Add in `+layout.svelte`'s `onMount`:
```ts
const unlistenPeerRemoved = listen<string>('peer_removed', (event) => {
  peers.update(list => list.filter(p => p.id !== event.payload));
});
// and cleanup: unlistenPeerRemoved.then(fn => fn());
```

---

### 1.8 `peers` table in DB is written to schema but never populated

**File:** `src-tauri/src/db.rs` creates the `peers` table. Nothing ever inserts into it.

**Fix:** In `mdns.rs` `ServiceResolved` handler, also upsert to DB:
```rust
conn.execute(
    "INSERT OR REPLACE INTO peers (id, name, ip, port, last_seen) VALUES (?1, ?2, ?3, ?4, ?5)",
    params![peer.id, peer.name, peer.ip, peer.port, peer.last_seen],
)?;
```
This enables "recently seen devices" history even if mDNS doesn't rediscover them on next launch.

---

### 1.9 `send_file` on devices page ignores the selected peer

**File:** `src/routes/+page.svelte`  
**Problem:**
```ts
function handleSend(peer: any) {
  goto('/send'); // peer is ignored completely
}
```
User clicks "Send File" on a specific device but arrives on `/send` with no pre-selection.

**Fix:** Pass peer ID via URL query param:
```ts
function handleSend(peer: Peer) {
  goto(`/send?peerId=${encodeURIComponent(peer.id)}`);
}
```
In `send/+page.svelte`, read it on mount:
```ts
import { page } from '$app/stores';
$effect(() => {
  const id = $page.url.searchParams.get('peerId');
  if (id) selectedPeerId = id;
});
```

---

### 1.10 `unwrap()` panics in `send_file` — no error propagation

**File:** `src-tauri/src/transfer.rs`, `send_file()` function  
**Problem:**
```rust
let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
let file_size = std::fs::metadata(&file_path).unwrap().len();
```
If file path has no filename segment or file has been deleted, this panics the entire app.

**Fix:** Convert `send_file` to return `Result<String, String>` and propagate errors as Tauri command errors:
```rust
#[tauri::command]
fn send_file(app: tauri::AppHandle, file_path: String, recipient_ip: String, port: u16) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file path")?
        .to_string();
    let file_size = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .len();
    // ...
    Ok(transfer_id)
}
```

---

### 1.11 `tokio-tungstenite` dep declared but unused

**File:** `src-tauri/Cargo.toml`  
**Problem:** `tokio-tungstenite = "0.23"` is in dependencies but no code imports or uses it. Increases compile time and binary size.

**Fix:** Remove from `Cargo.toml` unless WebSocket support is actively being implemented.

---

### 1.12 CSP is `null` — security misconfiguration

**File:** `src-tauri/tauri.conf.json`  
```json
"security": { "csp": null }
```
Disabling CSP allows arbitrary script injection via XSS.

**Fix:**
```json
"security": {
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: https://asset.localhost"
}
```

---

## SECTION 2 — SECURITY ISSUES

### 2.1 No transfer size limit — disk exhaustion attack

**File:** `src-tauri/src/transfer.rs`  
**Problem:** `handle_incoming_transfer` streams bytes until `bytes_received < request.file_size` with no maximum cap. A malicious peer can advertise a 1TB file and exhaust disk.

**Fix:** Add a configurable max file size (default 4GB):
```rust
const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4 GB

if request.file_size > MAX_FILE_SIZE {
    let _ = stream.write_all(b"{\"type\": \"reject\", \"reason\": \"file_too_large\"}").await;
    return;
}
```
Expose this as a user setting (see Settings section below).

### 2.2 mDNS service name uses unsanitized device name

**File:** `src-tauri/src/mdns.rs`  
```rust
let my_name = format!("{}._dropbridge._tcp.local.", devicename);
```
Device names with dots, underscores, or non-ASCII chars can break mDNS registration or be used to spoof service names.

**Fix:** Sanitize the device name before use:
```rust
let sanitized: String = devicename.chars()
    .filter(|c| c.is_alphanumeric() || *c == '-')
    .take(63) // DNS label max length
    .collect();
let my_name = format!("{}._dropbridge._tcp.local.", sanitized);
```

### 2.3 Admin dashboard is accessible by anyone — no gate

**File:** `src/routes/+layout.svelte`  
```ts
let isAdmin = $state(true); // always true
```
Admin section is hardcoded visible to everyone.

**Fix:** Implement a proper `claim_admin` flow. The existing stub command should: generate a local admin token, store it in the app's secure storage (`tauri-plugin-store` or keychain), and the frontend should check if the current user has the token before showing admin nav. For LAN context, a simple first-come-first-served PIN is sufficient.

---

## SECTION 3 — ARCHITECTURE IMPROVEMENTS

### 3.1 Settings are not persisted anywhere

**File:** `src/routes/settings/+page.svelte`  
**Problem:** All settings (`displayName`, `autoAccept`, `saveDirectory`, `notifications`) are ephemeral Svelte `$state`. They reset on every app restart.

**Fix:** 
1. Add a `settings` table to SQLite:
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```
2. Add Tauri commands:
```rust
#[tauri::command]
fn get_settings(state: tauri::State<'_, DbState>) -> Result<HashMap<String, String>, String> { ... }

#[tauri::command]
fn save_settings(state: tauri::State<'_, DbState>, settings: HashMap<String, String>) -> Result<(), String> { ... }
```
3. In `settings/+page.svelte`, call `invoke('get_settings')` on mount and `invoke('save_settings', { settings })` on save.
4. Wire `autoAccept` to `handle_incoming_transfer` in Rust: if the setting is true, skip the consent prompt and auto-accept.

---

### 3.2 Transfer cancellation: UI exists, backend does not

**File:** `src/routes/admin/+page.svelte` has a cancel button. No Tauri command exists for it.

**Fix:**
1. Add an `AbortHandle` or `CancellationToken` to `TransferState`:
```rust
pub struct TransferState {
    pub pending: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    pub active: Arc<tokio::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}
```
2. Add command:
```rust
#[tauri::command]
async fn cancel_transfer(state: tauri::State<'_, TransferState>, id: String) -> Result<(), String> {
    let mut active = state.active.lock().await;
    if let Some(tx) = active.remove(&id) {
        let _ = tx.send(());
    }
    Ok(())
}
```
3. In transfer loops, use `tokio::select!` to race file I/O against the cancel signal.

---

### 3.3 Multi-file send: only one progress bar tracked globally

**Problem:** `transfers` store tracks by `id`. When multiple files are queued and sent, progress events collide because the store update finds items by `id` but the UI doesn't differentiate per-file.

**Fix:** In `send/+page.svelte`, after `invoke('send_file')` returns an `id`, store per-file transfer records keyed by `id` and render individual progress bars per queued file in real time.

---

### 3.4 `discover_peers` command returns stale in-memory state

**Problem:** `discover_peers` returns the current `HashMap` snapshot. If called at startup before mDNS has had time to discover peers, it returns empty. The `onMount` call in layout does this synchronously.

**Fix:** Remove the synchronous `invoke('discover_peers')` initial call from layout. The real-time `peer_discovered` events are sufficient — mDNS events will populate the store within seconds of app start. Optionally, show a skeleton loader for 3 seconds on the Devices page.

---

### 3.5 Blocking `std::sync::Mutex` for `DbState.conn` — one DB connection for all

**Problem:** All DB operations serialize through one `Arc<Mutex<Connection>>`. This is fine for low concurrency but will cause lock contention if multiple transfers complete simultaneously (both try to log to DB at the same time).

**Fix:** Use a connection pool via `r2d2` + `r2d2_sqlite`, or use `rusqlite`'s `Connection::open_with_flags` with WAL mode enabled:
```rust
conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
```
WAL mode allows concurrent readers and one writer, dramatically reducing lock wait times.

---

## SECTION 4 — FEATURE ENHANCEMENTS

### 4.1 Transfer speed calculation missing

**Problem:** The history table and transfer progress have no speed (MB/s) display. The `TransferProgress` struct has no speed field.

**Fix:** Track timestamps in the progress emitter:
```rust
// In transfer.rs, add start_time tracking
let start = std::time::Instant::now();

// In the loop, emit speed:
let elapsed = start.elapsed().as_secs_f64();
let speed = if elapsed > 0.0 { bytes_received as f64 / elapsed } else { 0.0 };

app.emit("transfer_progress", TransferProgressWithSpeed {
    id: request.id.clone(),
    bytes_transferred: bytes_received,
    total_bytes: request.file_size,
    speed_bps: speed as u64,
})?;
```
Display `speed_bps` as "X MB/s" in the UI.

---

### 4.2 "Click to browse" missing on drop zone

**File:** `src/routes/send/+page.svelte`  
**Problem:** The drop zone is drag-only. Users can't click to open a file picker.

**Fix:** Add `tauri-plugin-dialog` to `Cargo.toml` and capabilities, then:
```ts
import { open } from '@tauri-apps/plugin-dialog';

async function browseFiles() {
  const selected = await open({ multiple: true, directory: false });
  if (selected) {
    const paths = Array.isArray(selected) ? selected : [selected];
    paths.forEach(p => {
      files.push({ name: p.split('/').pop() || p, size: 0, path: p });
    });
  }
}
```
Add a "Browse files" button inside the drop zone that calls `browseFiles()`.

---

### 4.3 Desktop notifications not implemented

**File:** `src/routes/settings/+page.svelte` has a `notifications` toggle with no effect.

**Fix:** Add `tauri-plugin-notification` to `Cargo.toml`. In `+layout.svelte`, when `transfer_requested` event fires, send a system notification:
```ts
import { sendNotification } from '@tauri-apps/plugin-notification';

listen('transfer_requested', (event) => {
  if (notificationsEnabled) {
    sendNotification({
      title: 'DropBridge — Incoming File',
      body: `${event.payload.sender} wants to send ${event.payload.file_name}`
    });
  }
});
```

---

### 4.4 Admin dashboard uses hardcoded mock data

**File:** `src/routes/admin/+page.svelte`  
All stats and active transfers are fake (`$state` with hardcoded values).

**Fix:**
1. Add `get_active_transfers` Tauri command that reads from `TransferState.active` (once implemented per 3.2).
2. Stats like `activePeers` should come from `DiscoveryState.peers.len()`.
3. `totalTransferred` should be a DB aggregate: `SELECT SUM(file_size) FROM transfers WHERE timestamp > ? AND status = 'completed'`.
4. Wire the Refresh button to re-invoke these commands.

---

### 4.5 Add transfer ETA display

In the progress events, calculate ETA:
```ts
const remaining = transfer.fileSize - (transfer.fileSize * transfer.progress / 100);
const etaSeconds = transfer.speedBps > 0 ? remaining / transfer.speedBps : null;
```
Display as "~2m 34s remaining" in the transfer progress row.

---

### 4.6 Add "Open file location" after completed receive

After a transfer completes on the receiver side, show an "Open Folder" button in the toast/history row. Use the existing `tauri-plugin-opener`:
```ts
import { openPath } from '@tauri-apps/plugin-opener';
await openPath(downloadDir); // reveal in Finder/Explorer
```

---

### 4.7 Clear history command

**File:** `src/routes/history/+page.svelte`  
No way to clear history exists.

**Fix:** Add Tauri command:
```rust
#[tauri::command]
fn clear_history(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM transfers", []).map_err(|e| e.to_string())?;
    Ok(())
}
```
Add a "Clear History" button to the history page header.

---

## SECTION 5 — FRONTEND / UX POLISH

### 5.1 Device type detection missing — all show generic icon

**File:** `src/routes/+page.svelte`  
`getIcon()` function exists but the `Peer` type has no `type` field — it's never populated by mDNS.

**Fix:** Add a `device_type` property to the mDNS TXT record:
```rust
let device_type = if cfg!(target_os = "macos") { "mac" }
    else if cfg!(target_os = "windows") { "windows" }
    else { "linux" };
let properties = [("name", username.as_str()), ("device_type", device_type)];
```
Parse it in `ServiceResolved` and include in `Peer`. Map in frontend:
```ts
function getIcon(type: string) {
  if (type === 'mac') return Laptop;
  if (type === 'windows') return Monitor;
  return Server;
}
```

---

### 5.2 Active nav item highlight uses wrong CSS class

**File:** `src/routes/+layout.svelte`  
```svelte
'bg-surface border-l-2 border-brand-translucent text-text-primary'
```
Active item uses `border-l-2` (left border) but `bg-surface` is the same as the sidebar background. The active state is nearly invisible.

**Fix:** Use a contrasting background for the active state:
```svelte
selectedPeerId === peer.id
  ? 'bg-brand/10 text-brand border-l-2 border-brand rounded-[6px]'
  : 'text-text-secondary hover:text-text-primary hover:bg-border-hover'
```

---

### 5.3 File size display uses raw `MB` — no smart formatting

**Problem:** `(file.size / (1024 * 1024)).toFixed(2) MB` shows `0.00 MB` for files under 1MB.

**Fix:** Add a shared utility:
```ts
// src/lib/utils.ts
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}
```
Use across `send/+page.svelte`, `history/+page.svelte`, and the consent modal.

---

### 5.4 Consent modal has no timeout — can block receiver indefinitely

**Problem:** If receiver ignores a transfer request, the sender's TCP connection stays open forever waiting for a response.

**Fix:** Add a 60-second timeout on the receiver side:
```rust
let accepted = tokio::time::timeout(
    std::time::Duration::from_secs(60),
    rx
).await.unwrap_or(Ok(false)).unwrap_or(false);
```
Emit a `transfer_timeout` event and clean up `pending` map entry.

On the frontend, show a countdown in the consent modal and auto-decline on timeout.

---

### 5.5 Scanning indicator always shows "Scanning..." — never updates

**File:** `src/routes/+page.svelte`  
The "Scanning..." badge is static — it never changes to "Idle" or shows peer count.

**Fix:**
```svelte
<div class="...">
  <div class="w-2 h-2 bg-brand rounded-full {$peers.length === 0 ? 'animate-pulse' : ''}"></div>
  <span class="...">
    {$peers.length === 0 ? 'Scanning...' : `${$peers.length} device${$peers.length !== 1 ? 's' : ''} found`}
  </span>
</div>
```

---

### 5.6 Settings "Save Changes" button does nothing

**File:** `src/routes/settings/+page.svelte`  
The save button exists but has no `onclick` handler.

**Fix:** Wire it to the persist command described in 3.1. Also show a success toast after saving.

---

### 5.7 Add a simple toast notification system

No toast/snackbar system exists. Errors and successes are silent.

**Fix:** Create `src/lib/components/Toast.svelte`:
```svelte
<script lang="ts">
  import { writable } from 'svelte/store';
  export const toasts = writable<{id: string, message: string, type: 'success'|'error'|'info'}[]>([]);

  export function toast(message: string, type: 'success'|'error'|'info' = 'info') {
    const id = crypto.randomUUID();
    toasts.update(t => [...t, { id, message, type }]);
    setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), 4000);
  }
</script>
```
Import and render in `+layout.svelte` as a fixed overlay. Call `toast()` on transfer events, errors, and settings save.

---

## SECTION 6 — CONFIGURATION & BUILD

### 6.1 Add `tauri-plugin-dialog` for native file picker
```toml
# Cargo.toml
tauri-plugin-dialog = "2"
```
```json
// capabilities/default.json — add:
"dialog:default"
```
```ts
// package.json devDependencies — add:
"@tauri-apps/plugin-dialog": "^2"
```

### 6.2 Add `tauri-plugin-notification` for system notifications
```toml
tauri-plugin-notification = "2"
```
```json
"notification:default"
```

### 6.3 Add `tauri-plugin-store` for persistent key-value settings (alternative to DB approach)
```toml
tauri-plugin-store = "2"
```
Use as a simpler alternative to the DB `settings` table for user preferences.

### 6.4 Enable WAL mode on SQLite at DB init
```rust
// In db.rs init_db(), after opening connection:
conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
```

### 6.5 Minimum window size
```json
// tauri.conf.json windows array:
"minWidth": 800,
"minHeight": 560,
```

### 6.6 Add app version to sidebar footer
```svelte
import { getVersion } from '@tauri-apps/api/app';
let version = $state('');
onMount(async () => { version = await getVersion(); });
```
Display as `v{version}` in the sidebar bottom.

---

## SECTION 7 — IMPLEMENTATION ORDER FOR GEMINI CLI

Implement in this exact sequence to avoid breaking interdependencies:

1. **Fix `lib.rs`** — remove dead code (1.1)
2. **Fix TCP framing** — length-prefixed protocol (1.3)
3. **Fix download path** — cross-platform (1.2)
4. **Fix `send_file` panics** — Result return type (1.10)
5. **Fix Mutex in async** — tokio::sync::Mutex (1.5)
6. **Remove dead SHA256 code** — (1.4, Option B)
7. **Remove unused tokio-tungstenite dep** — (1.11)
8. **Fix peer_removed listener** — (1.7)
9. **Fix history field mapping** — (1.6)
10. **Fix handleSend peer routing** — (1.9)
11. **Add settings persistence** — DB table + commands (3.1)
12. **Add cancel transfer** — (3.2)
13. **Add DB WAL mode** — (6.4)
14. **Add formatSize util** — (5.3)
15. **Add toast system** — (5.7)
16. **Add file browse button** — (4.2)
17. **Add notifications** — (4.3)
18. **Fix admin dashboard** — wire real data (4.4)
19. **Fix active nav highlight** — (5.2)
20. **Fix scanning badge** — (5.5)
21. **Fix settings save button** — (5.6)
22. **Add consent timeout** — (5.4)
23. **Add device type detection** — (5.1)
24. **Add transfer speed + ETA** — (4.1, 4.5)
25. **Add "Open folder" button** — (4.6)
26. **Add clear history** — (4.7)
27. **Fix CSP** — (1.12)
28. **Fix mDNS sanitization** — (2.2)
29. **Add file size limit** — (2.1)
30. **Admin auth** — (2.3)

---

## QUICK REFERENCE — FILES CHANGED

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Delete or refactor |
| `src-tauri/src/main.rs` | Add settings/cancel commands, WAL, proper state types |
| `src-tauri/src/transfer.rs` | TCP framing, cross-platform path, Result returns, tokio Mutex, speed, cancellation, size limit, timeout |
| `src-tauri/src/db.rs` | WAL mode, settings table, clear history, DB peer upsert |
| `src-tauri/src/mdns.rs` | Device type TXT record, name sanitization |
| `src-tauri/Cargo.toml` | Remove tungstenite, add dialog/notification/store plugins |
| `src-tauri/tauri.conf.json` | CSP, min window size |
| `src-tauri/capabilities/default.json` | Add dialog/notification permissions |
| `src/lib/stores.ts` | Add speedBps, timestamp to Transfer type |
| `src/lib/utils.ts` | Create with formatSize |
| `src/lib/components/Toast.svelte` | Create toast system |
| `src/routes/+layout.svelte` | peer_removed listener, version display, toast import |
| `src/routes/+page.svelte` | Fix handleSend, fix badge, fix device icon |
| `src/routes/send/+page.svelte` | Peer pre-select from URL, browse button, speed display |
| `src/routes/history/+page.svelte` | Fix field mapping, timestamp column, clear button, speed |
| `src/routes/settings/+page.svelte` | Wire save button, load persisted settings |
| `src/routes/admin/+page.svelte` | Wire real data, wire cancel button |
