use serde::{Serialize, Deserialize};
use tokio::fs::{File, self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use tokio::sync::oneshot;
use std::sync::Arc;
use crate::{TransferState, DbState, db, TransferProgressWithSpeed};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferRequest {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sender: String,
}

const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4 GB

struct SpeedSampler {
    window: std::collections::VecDeque<(std::time::Instant, u64)>,
}

impl SpeedSampler {
    fn new() -> Self { Self { window: std::collections::VecDeque::new() } }

    fn record(&mut self, bytes: u64) {
        let now = std::time::Instant::now();
        self.window.push_back((now, bytes));
        while self.window.front().map(|(t, _)| t.elapsed().as_secs() > 3).unwrap_or(false) {
            self.window.pop_front();
        }
    }

    fn speed_bps(&self) -> u64 {
        if self.window.len() < 2 { return 0; }
        let (oldest_t, oldest_b) = self.window.front().unwrap();
        let (newest_t, newest_b) = self.window.back().unwrap();
        let elapsed = newest_t.duration_since(*oldest_t).as_secs_f64();
        if elapsed <= 0.0 { return 0; }
        ((newest_b - oldest_b) as f64 / elapsed) as u64
    }
}

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
    let mut buf = vec![0u8; 1024 * 1024]; // 1 MB
    let mut transferred = 0u64;
    let mut last_emit = std::time::Instant::now();
    let mut sampler = SpeedSampler::new();

    loop {
        let to_read = std::cmp::min(buf.len() as u64, total_bytes - transferred) as usize;
        if to_read == 0 {
            break;
        }

        tokio::select! {
            n_res = tokio::io::AsyncReadExt::read(&mut reader, &mut buf[..to_read]) => {
                match n_res {
                    Ok(0) => break,
                    Ok(n) => {
                        if tokio::io::AsyncWriteExt::write_all(&mut writer, &buf[..n]).await.is_err() {
                            return false;
                        }
                        transferred += n as u64;
                        sampler.record(transferred);
                        
                        if last_emit.elapsed().as_millis() > 200 {
                            let speed = sampler.speed_bps();
                            let _ = app.emit("transfer_progress", TransferProgressWithSpeed {
                                id: id.to_string(),
                                bytes_transferred: transferred,
                                total_bytes,
                                speed_bps: speed,
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

fn sanitize_filename(name: &str) -> String {
    let name = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("received_file");

    let sanitized: String = name.chars()
        .filter(|c| {
            !c.is_control() && 
            *c != '\0' &&
            !['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(c)
        })
        .collect();
    
    if sanitized.is_empty() {
        "received_file".to_string()
    } else {
        sanitized
    }
}

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

fn log_transfer_async(app: &AppHandle, direction: String, request: TransferRequest, status: String) {
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = app_clone.state::<DbState>().conn.lock() {
            let _ = db::log_transfer(&conn, &direction, &request, &status);
        }
    });
}

pub async fn start_listener(app: AppHandle) -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let port = listener.local_addr()?.port();
    let ip_counts = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::<std::net::IpAddr, u32>::new()));
    
    tauri::async_runtime::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    let app_handle = app.clone();
                    let ip = addr.ip();
                    let counts = ip_counts.clone();
                    
                    let mut counts_lock = counts.lock().await;
                    let count = counts_lock.entry(ip).or_insert(0);
                    if *count >= 3 {
                        println!("Rate limit exceeded for IP: {}", ip);
                        continue;
                    }
                    *count += 1;
                    drop(counts_lock);

                    tauri::async_runtime::spawn(async move {
                        handle_incoming_transfer(app_handle, &mut stream).await;
                        
                        let mut counts_lock = counts.lock().await;
                        if let Some(count) = counts_lock.get_mut(&ip) {
                            if *count > 0 {
                                *count -= 1;
                            }
                            if *count == 0 {
                                counts_lock.remove(&ip);
                            }
                        }
                    });
                }
                Err(e) => eprintln!("TCP listener error: {}", e),
            }
        }
    });

    Ok(port)
}

async fn handle_incoming_transfer(app: AppHandle, stream: &mut TcpStream) {
    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).await.is_err() { return; }
    let len = u32::from_le_bytes(len_buf) as usize;
    
    let mut json_buf = vec![0u8; len];
    if stream.read_exact(&mut json_buf).await.is_err() { return; }
    
    if let Ok(request) = serde_json::from_slice::<TransferRequest>(&json_buf) {
        if request.file_size > MAX_FILE_SIZE {
            let response = serde_json::json!({"type": "reject", "reason": "file_too_large"});
            let resp_json = serde_json::to_vec(&response).unwrap();
            let _ = stream.write_all(&(resp_json.len() as u32).to_le_bytes()).await;
            let _ = stream.write_all(&resp_json).await;
            return;
        }

        let mut auto_accept = false;
        let mut save_dir_opt = None;

        let db_conn = app.state::<DbState>().conn.clone();
        let settings = tokio::task::spawn_blocking(move || {
            let conn = db_conn.lock().ok()?;
            let auto_accept = conn.query_row(
                "SELECT value FROM settings WHERE key = 'autoAccept'",
                [],
                |row| row.get::<_, String>(0)
            ).map(|v| v == "true").unwrap_or(false);
            
            let save_dir = conn.query_row(
                "SELECT value FROM settings WHERE key = 'saveDirectory'",
                [],
                |row| row.get::<_, String>(0)
            ).ok();
            
            Some((auto_accept, save_dir))
        }).await.unwrap_or(None);

        if let Some((aa, sd)) = settings {
            auto_accept = aa;
            save_dir_opt = sd;
        }

        let accepted = if auto_accept {
            true
        } else {
            let (tx, rx) = oneshot::channel();
            {
                let state = app.state::<TransferState>();
                let mut pending = state.pending.lock().await;
                pending.insert(request.id.clone(), tx);
            }
            
            let _ = app.emit("transfer_requested", request.clone());
            
            match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
                Ok(Ok(val)) => val,
                _ => {
                    let state = app.state::<TransferState>();
                    let mut pending = state.pending.lock().await;
                    pending.remove(&request.id);
                    let _ = app.emit("transfer_timeout", request.id.clone());
                    false
                }
            }
        };
        
        if accepted {
            let response = serde_json::json!({"type": "accept"});
            let resp_json = serde_json::to_vec(&response).unwrap();
            let _ = stream.write_all(&(resp_json.len() as u32).to_le_bytes()).await;
            let _ = stream.write_all(&resp_json).await;
            
            let download_dir = save_dir_opt
                .map(PathBuf::from)
                .unwrap_or_else(|| app.path().download_dir().unwrap_or_else(|_| PathBuf::from(".")).join("DropBridge"));
            let _ = fs::create_dir_all(&download_dir).await;
            
            let safe_name = sanitize_filename(&request.file_name);
            let file_path = unique_path(&download_dir, &safe_name);
            
            let file = match File::create(&file_path).await {
                Ok(f) => f,
                Err(_) => return,
            };

            let (cancel_tx, cancel_rx) = oneshot::channel();
            {
                let state = app.state::<TransferState>();
                let mut active = state.active.lock().await;
                active.insert(request.id.clone(), cancel_tx);
            }

            let success = stream_with_progress(&app, &request.id, stream, file, request.file_size, cancel_rx).await;
            
            {
                let state = app.state::<TransferState>();
                let mut active = state.active.lock().await;
                active.remove(&request.id);
            }

            if success {
                let _ = app.emit("transfer_completed", request.id.clone());
                log_transfer_async(&app, "received".to_string(), request, "completed".to_string());
            } else {
                log_transfer_async(&app, "received".to_string(), request, "failed".to_string());
            }
        } else {
            let response = serde_json::json!({"type": "decline"});
            let resp_json = serde_json::to_vec(&response).unwrap();
            let _ = stream.write_all(&(resp_json.len() as u32).to_le_bytes()).await;
            let _ = stream.write_all(&resp_json).await;

            log_transfer_async(&app, "received".to_string(), request, "declined".to_string());
        }
    }
}

pub async fn send_file(
    app: AppHandle,
    file_path: PathBuf,
    recipient_ip: String,
    recipient_port: u16,
) -> Result<String, String> {
    let transfer_id = Uuid::new_v4().to_string();
    let file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")?
        .to_string();
    
    let meta = tokio::fs::metadata(&file_path).await.map_err(|e| e.to_string())?;
    let file_size = meta.len();
    
    if file_size > MAX_FILE_SIZE {
        return Err("File too large (max 4GB)".to_string());
    }

    let username = whoami::username().unwrap_or_else(|_| "Unknown".to_string());
    let request = TransferRequest {
        id: transfer_id.clone(),
        file_name,
        file_size,
        sender: username,
    };
    
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut stream = match TcpStream::connect(format!("{}:{}", recipient_ip, recipient_port)).await {
            Ok(s) => s,
            Err(e) => {
                let _ = app_handle.emit("transfer_failed", serde_json::json!({
                    "id": request.id,
                    "reason": e.to_string()
                }));
                log_transfer_async(&app_handle, "sent".to_string(), request, "failed".to_string());
                return;
            }
        };

        let _ = stream.set_nodelay(false);

        let request_json = serde_json::to_vec(&request).unwrap();
        let _ = stream.write_all(&(request_json.len() as u32).to_le_bytes()).await;
        let _ = stream.write_all(&request_json).await;
        
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() { return; }
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; len];
        if stream.read_exact(&mut resp_buf).await.is_err() { return; }
        
        let response: serde_json::Value = serde_json::from_slice(&resp_buf).unwrap_or_default();
        if response["type"] == "accept" {
            if let Ok(file) = File::open(&file_path).await {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                {
                    let state = app_handle.state::<TransferState>();
                    let mut active = state.active.lock().await;
                    active.insert(request.id.clone(), cancel_tx);
                }

                let success = stream_with_progress(&app_handle, &request.id, file, stream, request.file_size, cancel_rx).await;
                
                {
                    let state = app_handle.state::<TransferState>();
                    let mut active = state.active.lock().await;
                    active.remove(&request.id);
                }

                if success {
                    let _ = app_handle.emit("transfer_completed", request.id.clone());
                    log_transfer_async(&app_handle, "sent".to_string(), request, "completed".to_string());
                } else {
                    log_transfer_async(&app_handle, "sent".to_string(), request, "failed".to_string());
                }
            }
        } else {
            let _ = app_handle.emit("transfer_declined", request.id.clone());
            log_transfer_async(&app_handle, "sent".to_string(), request, "declined".to_string());
        }
    });
    
    Ok(transfer_id)
}