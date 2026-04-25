use serde::{Serialize, Deserialize};
use tokio::fs::{File, self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use tokio::sync::oneshot;
use crate::{TransferState, DbState, db};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferRequest {
    pub id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sender: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferProgress {
    pub id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
}

pub async fn start_listener(app: AppHandle) -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let port = listener.local_addr()?.port();
    
    println!("Transfer listener started on port: {}", port);
    
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
    let mut buffer = [0; 1024];
    let n = match stream.read(&mut buffer).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    
    if let Ok(request) = serde_json::from_slice::<TransferRequest>(&buffer[..n]) {
        let (tx, rx) = oneshot::channel();
        
        {
            let state = app.state::<TransferState>();
            let mut pending = state.pending.lock().unwrap();
            pending.insert(request.id.clone(), tx);
        }
        
        let _ = app.emit("transfer_requested", request.clone());
        let accepted = rx.await.unwrap_or(false);
        
        if accepted {
            let _ = stream.write_all(b"{\"type\": \"accept\"}").await;
            
            let download_dir = format!("/Users/{}/Downloads/DropBridge", whoami::username().unwrap_or_default());
            fs::create_dir_all(&download_dir).await.unwrap_or_default();
            
            let mut file = File::create(format!("{}/{}", download_dir, request.file_name)).await.unwrap();
            let mut bytes_received = 0;
            let mut buffer = [0; 65536];
            let mut hasher = Sha256::new();
            
            while bytes_received < request.file_size {
                let n = match stream.read(&mut buffer).await {
                    Ok(n) if n > 0 => n,
                    _ => break,
                };
                let _ = file.write_all(&buffer[..n]).await;
                hasher.update(&buffer[..n]);
                bytes_received += n as u64;
                
                let _ = app.emit("transfer_progress", TransferProgress {
                    id: request.id.clone(),
                    bytes_transferred: bytes_received,
                    total_bytes: request.file_size,
                });
            }
            
            let _ = app.emit("transfer_completed", request.id.clone());
            
            // Log to DB
            let db_state = app.state::<DbState>();
            let conn = db_state.conn.lock().unwrap();
            let _ = db::log_transfer(&conn, "received", &request, "completed");
        } else {
            let _ = stream.write_all(b"{\"type\": \"decline\"}").await;
            let db_state = app.state::<DbState>();
            let conn = db_state.conn.lock().unwrap();
            let _ = db::log_transfer(&conn, "received", &request, "declined");
        }
    }
}

pub fn send_file(
    app: AppHandle,
    file_path: PathBuf,
    recipient_ip: String,
    recipient_port: u16,
) -> String {
    let transfer_id = Uuid::new_v4().to_string();
    let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
    let file_size = std::fs::metadata(&file_path).unwrap().len();
    
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
            let _ = stream.write_all(&request_json).await;
            
            let mut buffer = [0; 1024];
            let n = match stream.read(&mut buffer).await {
                Ok(n) => n,
                Err(_) => return,
            };
            
            let response: serde_json::Value = serde_json::from_slice(&buffer[..n]).unwrap_or_default();
            if response["type"] == "accept" {
                if let Ok(mut file) = File::open(file_path).await {
                    let mut buffer = [0; 65536];
                    let mut bytes_sent = 0;
                    let mut hasher = Sha256::new();
                    
                    loop {
                        let n = match file.read(&mut buffer).await {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        if stream.write_all(&buffer[..n]).await.is_err() { break; }
                        hasher.update(&buffer[..n]);
                        bytes_sent += n as u64;
                        
                        let _ = app_handle.emit("transfer_progress", TransferProgress {
                            id: request.id.clone(),
                            bytes_transferred: bytes_sent,
                            total_bytes: file_size,
                        });
                    }
                    
                    let _ = app_handle.emit("transfer_completed", request.id.clone());
                    
                    let db_state = app_handle.state::<DbState>();
                    let conn = db_state.conn.lock().unwrap();
                    let _ = db::log_transfer(&conn, "sent", &request, "completed");
                }
            } else {
                let _ = app_handle.emit("transfer_declined", request.id.clone());
                let db_state = app_handle.state::<DbState>();
                let conn = db_state.conn.lock().unwrap();
                let _ = db::log_transfer(&conn, "sent", &request, "declined");
            }
        }
    });
    
    transfer_id
}
