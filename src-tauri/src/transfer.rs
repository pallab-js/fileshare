use serde::{Serialize, Deserialize};
use tokio::fs::{File, self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use tokio::sync::oneshot;
use crate::{TransferState, DbState, db, TransferProgressWithSpeed};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferRequest {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sender: String,
}

const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4 GB

pub async fn start_listener(app: AppHandle) -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let port = listener.local_addr()?.port();
    
    tauri::async_runtime::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_incoming_transfer(app_handle, &mut stream).await;
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
        {
            if let Ok(conn) = app.state::<DbState>().conn.lock() {
                if let Ok(mut stmt) = conn.prepare("SELECT value FROM settings WHERE key = 'autoAccept'") {
                    if let Ok(val) = stmt.query_row([], |row| row.get::<_, String>(0)) {
                        auto_accept = val == "true";
                    }
                }
            }
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
            
            let download_dir = app.path().download_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("DropBridge");
            let _ = fs::create_dir_all(&download_dir).await;
            
            let file_path = download_dir.join(&request.file_name);
            let mut file = match File::create(&file_path).await {
                Ok(f) => f,
                Err(_) => return,
            };

            let (cancel_tx, mut cancel_rx) = oneshot::channel();
            {
                let state = app.state::<TransferState>();
                let mut active = state.active.lock().await;
                active.insert(request.id.clone(), cancel_tx);
            }

            let mut bytes_received = 0;
            let mut buffer = [0; 65536];
            let start_time = std::time::Instant::now();
            let mut last_emit = std::time::Instant::now();
            
            let mut success = true;
            while bytes_received < request.file_size {
                let to_read = std::cmp::min(buffer.len() as u64, request.file_size - bytes_received) as usize;
                
                tokio::select! {
                    n_res = stream.read(&mut buffer[..to_read]) => {
                        match n_res {
                            Ok(n) if n > 0 => {
                                if file.write_all(&buffer[..n]).await.is_err() { 
                                    success = false;
                                    break; 
                                }
                                bytes_received += n as u64;
                                
                                if last_emit.elapsed().as_millis() > 200 {
                                    let elapsed = start_time.elapsed().as_secs_f64();
                                    let speed = if elapsed > 0.0 { bytes_received as f64 / elapsed } else { 0.0 };
                                    
                                    let _ = app.emit("transfer_progress", TransferProgressWithSpeed {
                                        id: request.id.clone(),
                                        bytes_transferred: bytes_received,
                                        total_bytes: request.file_size,
                                        speed_bps: speed as u64,
                                    });
                                    last_emit = std::time::Instant::now();
                                }
                            }
                            _ => { success = false; break; }
                        }
                    }
                    _ = &mut cancel_rx => {
                        success = false;
                        let _ = app.emit("transfer_cancelled", request.id.clone());
                        break;
                    }
                }
            }
            
            {
                let state = app.state::<TransferState>();
                let mut active = state.active.lock().await;
                active.remove(&request.id);
            }

            if success && bytes_received == request.file_size {
                let _ = app.emit("transfer_completed", request.id.clone());
                if let Ok(conn) = app.state::<DbState>().conn.lock() {
                    let _ = db::log_transfer(&conn, "received", &request, "completed");
                }
            } else {
                if let Ok(conn) = app.state::<DbState>().conn.lock() {
                    let _ = db::log_transfer(&conn, "received", &request, "failed");
                }
            }
        } else {
            let response = serde_json::json!({"type": "decline"});
            let resp_json = serde_json::to_vec(&response).unwrap();
            let _ = stream.write_all(&(resp_json.len() as u32).to_le_bytes()).await;
            let _ = stream.write_all(&resp_json).await;

            if let Ok(conn) = app.state::<DbState>().conn.lock() {
                let _ = db::log_transfer(&conn, "received", &request, "declined");
            }
        }
    }
}

pub fn send_file(
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
    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| e.to_string())?
        .len();
    
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
        if let Ok(mut stream) = TcpStream::connect(format!("{}:{}", recipient_ip, recipient_port)).await {
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
                if let Ok(mut file) = File::open(&file_path).await {
                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    {
                        let state = app_handle.state::<TransferState>();
                        let mut active = state.active.lock().await;
                        active.insert(request.id.clone(), cancel_tx);
                    }

                    let mut buffer = [0; 65536];
                    let mut bytes_sent = 0;
                    let start_time = std::time::Instant::now();
                    let mut last_emit = std::time::Instant::now();
                    
                    let mut success = true;
                    loop {
                        tokio::select! {
                            n_res = file.read(&mut buffer) => {
                                match n_res {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        if stream.write_all(&buffer[..n]).await.is_err() { 
                                            success = false;
                                            break; 
                                        }
                                        bytes_sent += n as u64;

                                        if last_emit.elapsed().as_millis() > 200 {
                                            let elapsed = start_time.elapsed().as_secs_f64();
                                            let speed = if elapsed > 0.0 { bytes_sent as f64 / elapsed } else { 0.0 };
                                            
                                            let _ = app_handle.emit("transfer_progress", TransferProgressWithSpeed {
                                                id: request.id.clone(),
                                                bytes_transferred: bytes_sent,
                                                total_bytes: file_size,
                                                speed_bps: speed as u64,
                                            });
                                            last_emit = std::time::Instant::now();
                                        }
                                    }
                                    Err(_) => { success = false; break; }
                                }
                            }
                            _ = &mut cancel_rx => {
                                success = false;
                                let _ = app_handle.emit("transfer_cancelled", request.id.clone());
                                break;
                            }
                        }
                    }
                    
                    {
                        let state = app_handle.state::<TransferState>();
                        let mut active = state.active.lock().await;
                        active.remove(&request.id);
                    }

                    if success {
                        let _ = app_handle.emit("transfer_completed", request.id.clone());
                        if let Ok(conn) = app_handle.state::<DbState>().conn.lock() {
                            let _ = db::log_transfer(&conn, "sent", &request, "completed");
                        }
                    } else {
                        if let Ok(conn) = app_handle.state::<DbState>().conn.lock() {
                            let _ = db::log_transfer(&conn, "sent", &request, "failed");
                        }
                    }
                }
            } else {
                let _ = app_handle.emit("transfer_declined", request.id.clone());
                if let Ok(conn) = app_handle.state::<DbState>().conn.lock() {
                    let _ = db::log_transfer(&conn, "sent", &request, "declined");
                }
            }
        }
    });
    
    Ok(transfer_id)
}
