# DropBridge — Full Codebase Upgrade & Fix Spec
> Feed this entire file into Gemini CLI. Implement changes in the order listed — each section depends on the previous.

---

## 0. Project Context

Tauri v2 desktop app (SvelteKit frontend + Rust backend). Transfers files over LAN via raw TCP. Peers discovered via mDNS. SQLite for history/settings.

Key files:
- `src-tauri/src/main.rs` — commands + app setup
- `src-tauri/src/transfer.rs` — TCP send/receive logic
- `src-tauri/src/mdns.rs` — mDNS registration + discovery
- `src-tauri/src/db.rs` — SQLite schema + queries
- `src/routes/+layout.svelte` — event listeners, nav, consent modal
- `src/routes/send/+page.svelte` — file picker + send UI
- `src/routes/history/+page.svelte` — transfer log
- `src/routes/settings/+page.svelte` — settings UI
- `src/routes/admin/+page.svelte` — admin dashboard
- `src/lib/stores.ts` — Svelte stores + types
- `src/lib/utils.ts` — formatting helpers

---

## 1. CRITICAL BUGS

### 1.1 File size always 0 on send page

**File:** `src/routes/send/+page.svelte`

`browseFiles()` pushes `size: 0` because the Tauri dialog returns paths, not file objects with metadata. The fix: add a Rust command to stat files after selection.

**Add to `src-tauri/src/main.rs`:**
```rust
#[tauri::command]
fn get_file_meta(path: String) -> Result<(String, u64), String> {
    let p = std::path::Path::new(&path);
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();
    Ok((name, meta.len()))
}
```
Register it in `invoke_handler`.

**Update `browseFiles()` in `send/+page.svelte`:**
```ts
async function browseFiles() {
  const selected = await open({ multiple: true, directory: false });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  for (const p of paths) {
    try {
      const [name, size] = await invoke<[string, number]>('get_file_meta', { path: p });
      files = [...files, { name, size, path: p }]; // reassign, don't mutate
    } catch (e: any) {
      addToast(e.toString(), 'error');
    }
  }
}
```

---

### 1.2 Svelte 5 rune state mutation (files array)

**File:** `src/routes/send/+page.svelte`

`files.push(...)` does NOT trigger reactivity in Svelte 5 rune mode. Every array write must reassign.

Replace ALL `files.push(...)` (in `browseFiles` and `handleDrop`) with:
```ts
files = [...files, newItem];
```

---

### 1.3 Speed field camelCase mismatch

**File:** `src/routes/+layout.svelte`, `src-tauri/src/main.rs`

`TransferProgressWithSpeed` has field `speed_bps`. Serde serializes this as `speed_bps` (snake_case) but the frontend reads `event.payload.speedBps` — undefined always.

**Fix in `main.rs`** — the struct already has `#[serde(rename_all = "camelCase")]` on `TransferProgressWithSpeed`, so the emitted JSON IS `speedBps`. But in `+layout.svelte`:

```ts
// WRONG:
t.speedBps = event.payload.speedBps;

// RIGHT (add type):
const prog = event.payload as { id: string; bytesTransferred: number; totalBytes: number; speedBps: number };
t.speedBps = prog.speedBps;
t.progress = (prog.bytesTransferred / prog.totalBytes) * 100;
```

Verify: `TransferProgressWithSpeed` in `main.rs` must have `#[serde(rename_all = "camelCase")]` — it does. Frontend was just untyped. Type it properly.

---

### 1.4 History page clobbers active transfers

**File:** `src/routes/history/+page.svelte`

`loadHistory()` calls `transfers.set(mapped)` — this destroys any in-progress transfers in the store.

**Fix:** Use a separate `historyItems` local state, never touch the global `transfers` store from history page:

```ts
// In history/+page.svelte script:
let historyItems = $state<any[]>([]);

async function loadHistory() {
  try {
    const history = await invoke<any[]>('get_history');
    historyItems = history.map(h => ({ ...h }));
  } catch (e) { console.error(e); }
}
```

Update the template to use `historyItems` instead of `$transfers`.

---

### 1.5 Own device appears in peer list (self-discovery)

**File:** `src-tauri/src/mdns.rs`

mDNS resolves own registration, adding self to peers.

**Fix in `start_discovery`, inside `ServiceEvent::ServiceResolved` handler:**
```rust
// After getting ip:
let my_ip_str = local_ip_address::local_ip()
    .map(|ip| ip.to_string())
    .unwrap_or_default();
if ip == my_ip_str {
    continue; // skip self
}
```

---

### 1.6 Drag-and-drop file path on Web/Tauri

**File:** `src/routes/send/+page.svelte`

`(f as any).path` is only available in Tauri's webview, not standard. Verify with a fallback:

```ts
function handleDrop(e: DragEvent) {
  e.preventDefault();
  isDragging = false;
  const droppedFiles = Array.from(e.dataTransfer?.files ?? []);
  files = [
    ...files,
    ...droppedFiles.map(f => ({
      name: f.name,
      size: f.size,
      path: (f as any).path ?? f.name, // Tauri injects .path
    }))
  ];
}
```

---

## 2. SECURITY ISSUES

### 2.1 No encryption on transfer

**File:** `src-tauri/src/transfer.rs`

All file data is sent plaintext over TCP. On a trusted LAN this may be acceptable but should at least be documented. Recommended fix: wrap `TcpStream` with `tokio-rustls` using a self-signed cert per session.

**Minimal approach (add to `Cargo.toml`):**
```toml
tokio-rustls = "0.26"
rustls = { version = "0.23", features = ["ring"] }
rcgen = "0.13"
```

Generate ephemeral self-signed cert on startup, include public key fingerprint in mDNS TXT records so peers can pin it. Full impl is out of scope here but the plaintext TCP should be flagged prominently in README as a known limitation.

---

### 2.2 `claim_admin` is a no-op

**File:** `src-tauri/src/main.rs`, `src/routes/+layout.svelte`

`claim_admin` returns `Ok(())` unconditionally. `isAdmin` in layout is hardcoded `true`. Admin page is always accessible.

**Fix:**
- Remove `claim_admin` command entirely OR implement a real check (e.g. PIN from settings)
- In `+layout.svelte`, remove `let isAdmin = $state(true)` and instead gate admin nav item behind a settings flag or remove the admin page from sidebar for now
- Admin page itself has no auth guard

---

### 2.3 No connection rate limiting

**File:** `src-tauri/src/transfer.rs`

`start_listener` accepts unlimited concurrent TCP connections. A malicious peer can open thousands.

**Fix — add a semaphore:**
```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn start_listener(app: AppHandle) -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let port = listener.local_addr()?.port();
    let sem = Arc::new(Semaphore::new(10)); // max 10 concurrent transfers

    tauri::async_runtime::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let app_handle = app.clone();
                    let permit = sem.clone().acquire_owned().await.unwrap();
                    tauri::async_runtime::spawn(async move {
                        handle_incoming_transfer(app_handle, &mut stream).await;
                        drop(permit);
                    });
                }
                Err(e) => eprintln!("TCP listener error: {}", e),
            }
        }
    });
    Ok(port)
}
```

---

### 2.4 Path traversal in received filename

**File:** `src-tauri/src/transfer.rs`

`request.file_name` comes from the sender over the network. A malicious sender can set `file_name = "../../.bashrc"`.

**Fix — sanitize filename before writing:**
```rust
use std::path::Path;

fn sanitize_filename(name: &str) -> String {
    Path::new(name)
        .file_name()                          // strip any directory component
        .and_then(|n| n.to_str())
        .unwrap_or("received_file")
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

// In handle_incoming_transfer, replace:
let file_path = download_dir.join(&request.file_name);
// With:
let safe_name = sanitize_filename(&request.file_name);
let file_path = download_dir.join(&safe_name);
```

---

### 2.5 File collision — silent overwrite

**File:** `src-tauri/src/transfer.rs`

If a file with the same name already exists in `Downloads/DropBridge`, it is silently overwritten.

**Fix — append counter if file exists:**
```rust
fn unique_path(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    if !p.exists() { return p; }
    let stem = std::path::Path::new(name)
        .file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = std::path::Path::new(name)
        .extension().and_then(|e| e.to_str()).unwrap_or("");
    for i in 1..=999 {
        let candidate = if ext.is_empty() {
            dir.join(format!("{} ({})", stem, i))
        } else {
            dir.join(format!("{} ({}).{}", stem, i, ext))
        };
        if !candidate.exists() { return candidate; }
    }
    dir.join(format!("{}-{}", stem, uuid::Uuid::new_v4()))
}

// Usage:
let file_path = unique_path(&download_dir, &safe_name);
```

---

## 3. ARCHITECTURE / DESIGN ISSUES

### 3.1 Massive code duplication in transfer loop

**File:** `src-tauri/src/transfer.rs`

The send loop and receive loop are nearly identical (~80 lines each). Extract a shared progress-emitting copy helper:

```rust
async fn stream_with_progress<R, W>(
    app: &AppHandle,
    id: &str,
    mut reader: R,
    mut writer: W,
    total_bytes: u64,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> bool
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut buf = [0u8; 65536];
    let mut transferred = 0u64;
    let start = std::time::Instant::now();
    let mut last_emit = std::time::Instant::now();

    loop {
        tokio::select! {
            n_res = tokio::io::AsyncReadExt::read(&mut reader, &mut buf) => {
                match n_res {
                    Ok(0) => break,
                    Ok(n) => {
                        if tokio::io::AsyncWriteExt::write_all(&mut writer, &buf[..n]).await.is_err() {
                            return false;
                        }
                        transferred += n as u64;
                        if last_emit.elapsed().as_millis() > 200 {
                            let elapsed = start.elapsed().as_secs_f64();
                            let speed = if elapsed > 0.0 { transferred as f64 / elapsed } else { 0.0 };
                            let _ = app.emit("transfer_progress", crate::TransferProgressWithSpeed {
                                id: id.to_string(),
                                bytes_transferred: transferred,
                                total_bytes,
                                speed_bps: speed as u64,
                            });
                            last_emit = std::time::Instant::now();
                        }
                        if transferred >= total_bytes { break; }
                    }
                    Err(_) => return false,
                }
            }
            _ = &mut cancel_rx => {
                let _ = app.emit("transfer_cancelled", id);
                return false;
            }
        }
    }
    transferred >= total_bytes
}
```

Replace both loops in `handle_incoming_transfer` and `send_file` with calls to this function.

---

### 3.2 Speed shows cumulative average, not recent speed

**File:** `src-tauri/src/transfer.rs`

`speed = bytes_transferred / total_elapsed` gives an average from transfer start, not recent throughput. Looks stale at end of transfer.

**Fix — use a rolling window:**
```rust
struct SpeedSampler {
    window: std::collections::VecDeque<(std::time::Instant, u64)>,
}

impl SpeedSampler {
    fn new() -> Self { Self { window: std::collections::VecDeque::new() } }

    fn record(&mut self, bytes: u64) {
        let now = std::time::Instant::now();
        self.window.push_back((now, bytes));
        // Keep only last 3 seconds
        while self.window.front().map(|(t, _)| t.elapsed().as_secs() > 3).unwrap_or(false) {
            self.window.pop_front();
        }
    }

    fn speed_bps(&self) -> u64 {
        if self.window.len() < 2 { return 0; }
        let (oldest_t, oldest_b) = self.window.front().unwrap();
        let (newest_t, newest_b) = self.window.back().unwrap();
        let elapsed = newest_t.duration_since(*oldest_t).as_secs_f64();
        if elapsed == 0.0 { return 0; }
        ((newest_b - oldest_b) as f64 / elapsed) as u64
    }
}
```

Use in the transfer loop instead of the current `start_time` approach.

---

### 3.3 SQLite mutex deadlock risk in async context

**File:** `src-tauri/src/main.rs`, throughout

`DbState.conn` uses `std::sync::Mutex`. Locking a `std::sync::Mutex` inside an async function blocks the executor thread.

**Fix:** Either:
- (A) Use `tokio::sync::Mutex` for `DbState.conn` and `.await` the lock, OR
- (B) Keep `std::sync::Mutex` but wrap ALL DB operations in `tokio::task::spawn_blocking`

Option B is simpler and keeps rusqlite (non-Send) usage contained:

```rust
// Example in transfer.rs:
let conn_arc = app.state::<DbState>().conn.clone();
tokio::task::spawn_blocking(move || {
    if let Ok(conn) = conn_arc.lock() {
        let _ = db::log_transfer(&conn, "received", &request, "completed");
    }
}).await.ok();
```

Apply this pattern everywhere `conn.lock()` is called inside async fns.

---

### 3.4 DB migration is fragile

**File:** `src-tauri/src/db.rs`

Schema migrations use `PRAGMA table_info` checks — breaks if column added in wrong order or if schema diverges between app versions.

**Fix — use `user_version` pragma for versioned migrations:**
```rust
pub fn init_db(app: &AppHandle) -> Result<Connection, rusqlite::Error> {
    let conn = /* ... open connection ... */;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    
    if version < 1 {
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS peers (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, ip TEXT NOT NULL,
                port INTEGER NOT NULL, last_seen INTEGER NOT NULL, device_type TEXT NOT NULL DEFAULT 'unknown'
            );
            CREATE TABLE IF NOT EXISTS transfers (
                id TEXT PRIMARY KEY, direction TEXT NOT NULL, peer_name TEXT NOT NULL,
                file_name TEXT NOT NULL, file_size INTEGER NOT NULL,
                status TEXT NOT NULL, timestamp INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            PRAGMA user_version = 1;
        ")?;
    }
    // Future: if version < 2 { /* add column */ PRAGMA user_version = 2; }
    
    Ok(conn)
}
```

Remove all the `PRAGMA table_info` migration blocks.

---

### 3.5 Stale peers never cleaned from DB

**File:** `src-tauri/src/db.rs`, `src-tauri/src/mdns.rs`

`peers` table accumulates entries forever. Old peers with stale IPs/ports mislead.

**Fix — purge peers older than 24h on startup:**
```rust
// In init_db, after schema setup:
conn.execute(
    "DELETE FROM peers WHERE last_seen < ?1",
    params![chrono::Utc::now().timestamp() - 86400],
)?;
```

Also add index for timestamp lookups:
```sql
CREATE INDEX IF NOT EXISTS idx_transfers_timestamp ON transfers(timestamp DESC);
```

---

### 3.6 `displayName` setting never used in mDNS

**File:** `src-tauri/src/mdns.rs`, `src-tauri/src/main.rs`

User sets a display name in Settings but mDNS always broadcasts `whoami::username()`. The setting is saved to DB but never read at registration time.

**Fix:** Read `displayName` from DB before registering mDNS service. Pass it through `start_discovery`:

```rust
// In main.rs setup:
let display_name = {
    if let Ok(conn) = db_conn.lock() {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'displayName'",
            [],
            |r| r.get::<_, String>(0)
        ).ok()
    } else { None }
}.unwrap_or_else(|| whoami::username().unwrap_or_else(|_| "Unknown".to_string()));

mdns::start_discovery(handle, port, display_name).expect("failed to start mDNS");

// Update start_discovery signature:
pub fn start_discovery(app: AppHandle, transfer_port: u16, display_name: String) -> Result<(), ...>
// Use display_name instead of whoami::username() in properties
```

---

## 4. UI/UX ENHANCEMENTS

### 4.1 Settings: "Change" save directory button is broken

**File:** `src/routes/settings/+page.svelte`

The "Change" button next to save directory does nothing.

**Fix:**
```ts
// Add import at top:
import { open } from '@tauri-apps/plugin-dialog';

// Add function:
async function changeSaveDir() {
  const dir = await open({ directory: true, multiple: false });
  if (dir && typeof dir === 'string') {
    settings.saveDirectory = dir;
  }
}
```

Update button:
```svelte
<button onclick={changeSaveDir} class="...">Change</button>
```

Also: the `saveDirectory` setting is stored but `transfer.rs` hardcodes `downloads_dir/DropBridge`. Wire it up:

In `handle_incoming_transfer`, read from settings:
```rust
let save_dir = {
    if let Ok(conn) = app.state::<DbState>().conn.lock() {
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'saveDirectory'",
            [], |r| r.get::<_, String>(0)
        ).ok().map(PathBuf::from)
    } else { None }
}.unwrap_or_else(|| {
    app.path().download_dir().unwrap_or_else(|_| PathBuf::from(".")).join("DropBridge")
});
```

---

### 4.2 Admin audit log is static/hardcoded

**File:** `src/routes/admin/+page.svelte`

Audit log shows fake hardcoded entries ("Admin session initialized", "mDNS browsing active").

**Fix A (minimal):** Remove fake log entries. Show `$transfers` last 10 events as the audit trail.

**Fix B (proper):** Add an `audit_log` table to SQLite:
```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event TEXT NOT NULL,
    detail TEXT,
    timestamp INTEGER NOT NULL
);
```

Add Rust command:
```rust
#[tauri::command]
fn get_audit_log(state: tauri::State<'_, DbState>) -> Result<Vec<AuditEntry>, String> { ... }
```

Log events: peer discovered, transfer started, transfer completed/failed/cancelled, settings saved.

---

### 4.3 No transfer queue progress on home/devices page

Currently active transfers only appear on the Send page. A user on the Devices page has no feedback.

**Fix:** Add a persistent active-transfers tray to `+layout.svelte` — a slim bottom bar that shows active transfer count and aggregate speed when `$transfers.some(t => t.status === 'streaming')`:

```svelte
{#if $transfers.some(t => t.status === 'streaming')}
  <div class="fixed bottom-0 left-[240px] right-0 bg-surface border-t border-border px-6 py-3 flex items-center justify-between z-40">
    <div class="flex items-center gap-3">
      <div class="w-2 h-2 bg-brand rounded-full animate-pulse"></div>
      <span class="text-sm font-medium">
        {$transfers.filter(t => t.status === 'streaming').length} transfer(s) active
      </span>
    </div>
    <span class="text-xs font-mono text-brand">
      {formatSpeed($transfers.reduce((acc, t) => acc + (t.speedBps ?? 0), 0))}
    </span>
  </div>
{/if}
```

---

### 4.4 History page: add pagination and filter

**File:** `src/routes/history/+page.svelte`

History loads all records with no limit. Add:
- Page size = 50, pagination controls
- Status filter buttons: All / Completed / Failed / Cancelled

```ts
let filterStatus = $state<string>('all');
let page = $state(0);
const PAGE_SIZE = 50;

let filteredItems = $derived(
  historyItems
    .filter(h => filterStatus === 'all' || h.status === filterStatus)
    .slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE)
);
```

Add to Rust command: `get_history` should accept `limit` + `offset` params:
```rust
#[tauri::command]
fn get_history(state: tauri::State<'_, DbState>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransferHistory>, String> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    // "... ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
}
```

---

### 4.5 No notification on transfer completion

**File:** `src/routes/+layout.svelte`

`tauri_plugin_notification` is registered but never called. On transfer completion, trigger a native desktop notification:

```ts
import { sendNotification } from '@tauri-apps/plugin-notification';

// In unlistenCompleted handler:
const unlistenCompleted = listen<string>('transfer_completed', (event) => {
  transfers.update(list => {
    const t = list.find(x => x.id === event.payload);
    if (t) {
      t.progress = 100;
      t.status = 'completed';
      addToast(`Transfer completed: ${t.fileName}`, 'success');
      if (settings.notifications) {  // check user pref
        sendNotification({ title: 'DropBridge', body: `${t.fileName} received successfully.` });
      }
    }
    return [...list];
  });
});
```

Read `notifications` setting from invoke on mount and store in a local variable.

---

### 4.6 ETA shown but `formatETA` never called

**File:** `src/routes/send/+page.svelte`, `src/lib/utils.ts`

`formatETA` is imported in `send/+page.svelte` but never used in template. Add ETA display to active transfer rows in the queue section:

```svelte
{#if t.status === 'streaming'}
  <p class="text-xs font-mono text-text-muted">
    {formatSpeed(t.speedBps ?? 0)} · ETA {formatETA(t.fileSize * (1 - t.progress/100), t.speedBps ?? 0)}
  </p>
{/if}
```

---

## 5. PERFORMANCE IMPROVEMENTS

### 5.1 Increase TCP buffer size

**File:** `src-tauri/src/transfer.rs`

Buffer is 64 KB (`[0; 65536]`). On fast LAN (1 Gbps), this causes excessive syscall overhead. Increase to 256 KB or 1 MB:

```rust
let mut buffer = vec![0u8; 1024 * 1024]; // 1 MB
```

Also set TCP socket options for better throughput:

```rust
use std::net::TcpStream as StdTcpStream;

// After TcpStream::connect:
let std_stream = stream.into_std()?;
std_stream.set_nodelay(false)?;      // Allow Nagle for bulk transfer
let stream = TcpStream::from_std(std_stream)?;
```

---

### 5.2 Async file I/O on send side

**File:** `src-tauri/src/transfer.rs`

`send_file` uses `std::fs::metadata` synchronously before spawning the async task. Move it inside the async block or use `tokio::fs::metadata`:

```rust
// In send_file async block, replace:
let file_size = std::fs::metadata(&file_path).map_err(|e| e.to_string())?.len();
// With:
let meta = tokio::fs::metadata(&file_path).await.map_err(|e| e.to_string())?;
let file_size = meta.len();
```

But `send_file` is a sync `#[tauri::command]`. Convert it to async:
```rust
#[tauri::command]
async fn send_file(app: tauri::AppHandle, file_path: String, recipient_ip: String, port: u16) -> Result<String, String> {
    transfer::send_file(app, PathBuf::from(file_path), recipient_ip, port).await
}
```

And make `transfer::send_file` async, removing the inner `tauri::async_runtime::spawn`.

---

### 5.3 Emit throttle is per-transfer but shared timer

**File:** `src-tauri/src/transfer.rs`

With multiple concurrent transfers, `last_emit` is local so each has its own 200ms window — this is fine. No change needed here, but ensure `stream_with_progress` (from fix 3.1) keeps `last_emit` scoped per-call.

---

## 6. CODE QUALITY

### 6.1 Remove unused imports

**File:** `src/routes/send/+page.svelte`

`Search` is imported from `lucide-svelte` but unused. Remove it.

**File:** `src/routes/history/+page.svelte`

`formatSpeed` and `formatETA` imported but unused. Remove or use them (see fix 4.6).

---

### 6.2 Type all Tauri event payloads

**File:** `src/routes/+layout.svelte`

Multiple `listen<any>` calls. Define proper types in `stores.ts` and use them:

```ts
// In stores.ts, add:
export interface TransferRequestPayload {
  id: string;
  file_name: string;
  file_size: number;
  sender: string;
}

export interface TransferProgressPayload {
  id: string;
  bytesTransferred: number;
  totalBytes: number;
  speedBps: number;
}
```

Update `listen<any>` to `listen<TransferRequestPayload>` etc.

---

### 6.3 Hardcoded `isAdmin = true`

**File:** `src/routes/+layout.svelte`

```ts
let isAdmin = $state(true); // This is never false
```

Either implement real admin auth (settings PIN) or remove admin entirely until implemented. Shipping a permanently-unlocked admin panel is misleading. Minimum fix:

```ts
// Remove isAdmin entirely, remove admin nav item from navItems or add it unconditionally
```

---

### 6.4 `whoami` error handling

**File:** `src-tauri/src/mdns.rs`, `src-tauri/src/transfer.rs`

`whoami::username().unwrap_or_else(|_| "Unknown".to_string())` — fine. But `whoami::devicename()` can return a hostname with Unicode or special chars that break mDNS. The sanitizer already handles this but doesn't enforce minimum length:

```rust
let sanitized: String = devicename.chars()
    .filter(|c| c.is_alphanumeric() || *c == '-')
    .take(63)
    .collect();
let sanitized = if sanitized.is_empty() { "DropBridge".to_string() } else { sanitized };
```

---

### 6.5 Error on TCP connect failure is silently swallowed

**File:** `src-tauri/src/transfer.rs`

In `send_file`, if `TcpStream::connect` fails, the async task just returns silently. The user sees the transfer as "pending" forever.

**Fix:**
```rust
let mut stream = match TcpStream::connect(format!("{}:{}", recipient_ip, recipient_port)).await {
    Ok(s) => s,
    Err(e) => {
        let _ = app_handle.emit("transfer_failed", serde_json::json!({
            "id": request.id,
            "reason": e.to_string()
        }));
        if let Ok(conn) = app_handle.state::<DbState>().conn.lock() {
            let _ = db::log_transfer(&conn, "sent", &request, "failed");
        }
        return;
    }
};
```

Add `transfer_failed` listener in `+layout.svelte` to update transfer status.

---

## 7. MISSING FEATURES (Implement in priority order)

### 7.1 Transfer resume / integrity check

After transfer, verify file integrity with SHA-256:
- Sender computes hash of file before sending, includes it in `TransferRequest`
- Receiver computes hash of received bytes, emits `transfer_integrity_ok` or `transfer_corrupted`

```rust
// In TransferRequest:
pub checksum: Option<String>, // SHA-256 hex

// In send_file, before sending:
use sha2::{Sha256, Digest};
let mut hasher = Sha256::new();
// read file, update hasher
let checksum = format!("{:x}", hasher.finalize());
```

---

### 7.2 Multi-file batch transfer

Currently each file opens a new TCP connection. For batch sends, either:
- (A) Multiplex files over one connection with a framing protocol
- (B) Queue and send sequentially over separate connections (simpler)

Option B is already half-done (`sendFiles` loops). The issue is the UI shows all as "pending" and there's no sequencing. Add a queue manager in the store that processes one at a time and shows "queued" status for waiting files.

---

### 7.3 Clipboard / text snippet sharing

Add a "Send Text" mode alongside "Send File" — paste text, it gets transferred as a `.txt` file or displayed directly in a modal on the receiver's end.

---

### 7.4 Dark/light theme toggle

`app.css` likely has a dark theme hardcoded. Add a `theme` setting (dark/light/system) and toggle CSS class on `<html>`.

---

## 8. IMPLEMENTATION ORDER FOR GEMINI CLI

Apply fixes in this sequence to avoid conflicts:

1. **Section 1** — all critical bugs (no dependencies)
2. **Section 2.4** — path traversal (security, touches transfer.rs)
3. **Section 2.5** — file collision (touches transfer.rs, same area)
4. **Section 3.4** — DB migration (db.rs, do before other DB changes)
5. **Section 3.3** — mutex fix (requires knowing all async DB call sites)
6. **Section 3.1** — deduplicate transfer loop (refactor, do after 3.3)
7. **Section 3.2** — rolling speed (add to new shared fn from 3.1)
8. **Section 2.3** — semaphore rate limiting (touches start_listener)
9. **Section 3.6** — displayName in mDNS (requires DB read at startup)
10. **Section 3.5** — stale peer cleanup (small DB change)
11. **Section 5.2** — async send_file (refactor, do after 3.1)
12. **Section 4.x** — all UI enhancements (frontend only, no Rust deps)
13. **Section 6.x** — code quality cleanup (last, non-breaking)
14. **Section 7.x** — new features (after all fixes stable)

---

*End of spec. All code snippets are illustrative — Gemini should adapt them to exact current file content.*
