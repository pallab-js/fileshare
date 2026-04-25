// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mdns;
mod transfer;
mod db;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, Emitter};
use crate::mdns::{Peer, DiscoveryState};
use std::path::PathBuf;
use tokio::sync::oneshot;
use rusqlite::{Connection, params};
use serde::Serialize;

pub struct DbState {
    pub conn: Arc<Mutex<Connection>>,
}

pub struct TransferState {
    pub pending: Arc<tokio::sync::Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    pub active: Arc<tokio::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistory {
    pub id: String,
    pub direction: String,
    pub peer_name: String,
    pub file_name: String,
    pub file_size: u64,
    pub status: String,
    pub timestamp: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgressWithSpeed {
    pub id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

#[tauri::command]
fn discover_peers(state: tauri::State<'_, DiscoveryState>) -> Vec<Peer> {
    let peers = state.peers.lock().unwrap();
    peers.values().cloned().collect()
}

#[tauri::command]
async fn send_file(
    app: tauri::AppHandle,
    file_path: String,
    recipient_ip: String,
    port: u16,
) -> Result<String, String> {
    transfer::send_file(app, PathBuf::from(file_path), recipient_ip, port).await
}

#[tauri::command]
async fn respond_to_transfer(
    state: tauri::State<'_, TransferState>,
    id: String,
    accept: bool,
) -> Result<(), String> {
    let mut pending = state.pending.lock().await;
    if let Some(tx) = pending.remove(&id) {
        let _ = tx.send(accept);
    }
    Ok(())
}

#[tauri::command]
async fn cancel_transfer(state: tauri::State<'_, TransferState>, id: String) -> Result<(), String> {
    let mut active = state.active.lock().await;
    if let Some(tx) = active.remove(&id) {
        let _ = tx.send(());
    }
    Ok(())
}

#[tauri::command]
async fn get_history(state: tauri::State<'_, DbState>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransferHistory>, String> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let conn = state.conn.clone();
    
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, direction, peer_name, file_name, file_size, status, timestamp FROM transfers ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| e.to_string())?;
        
        let history_iter = stmt.query_map(params![limit, offset], |row| {
            Ok(TransferHistory {
                id: row.get(0)?,
                direction: row.get(1)?,
                peer_name: row.get(2)?,
                file_name: row.get(3)?,
                file_size: row.get(4)?,
                status: row.get(5)?,
                timestamp: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut history = Vec::new();
        for item in history_iter {
            history.push(item.map_err(|e| e.to_string())?);
        }
        
        Ok(history)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clear_history(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.clone();
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM transfers", []).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, DbState>) -> Result<HashMap<String, String>, String> {
    let conn = state.conn.clone();
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;

        let mut settings = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|e| e.to_string())?;
            settings.insert(key, value);
        }
        Ok(settings)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_settings(state: tauri::State<'_, DbState>, settings: HashMap<String, String>) -> Result<(), String> {
    let conn = state.conn.clone();
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        for (key, value) in settings {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn direct_connect(
    app: tauri::AppHandle,
    state: tauri::State<'_, DiscoveryState>,
    ip: String,
    port: u16,
) -> Result<Peer, String> {
    // Attempt to connect to verify it's a DropBridge instance
    // For now, we just check if the port is open and maybe in the future 
    // we can do a proper handshake to get the name.
    // For simplicity, we'll try to connect and if it works, we add it.
    
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let _stream = timeout(Duration::from_secs(3), TcpStream::connect(format!("{}:{}", ip, port)))
        .await
        .map_err(|_| "Connection timed out".to_string())?
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let peer = Peer {
        id: format!("manual:{}", ip),
        name: format!("Manual: {}", ip),
        ip: ip.clone(),
        port,
        last_seen: chrono::Utc::now().timestamp(),
        device_type: "unknown".to_string(),
    };

    {
        let mut peers = state.peers.lock().unwrap();
        peers.insert(peer.id.clone(), peer.clone());
    }
    
    let _ = app.emit("peer_discovered", peer.clone());
    Ok(peer)
}

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

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(DiscoveryState {
            peers: Arc::new(Mutex::new(HashMap::new())),
            mdns: Mutex::new(None),
            service_name: Mutex::new(None),
        })
        .manage(TransferState {
            pending: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            active: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Initialize DB
            let conn = db::init_db(&handle).expect("failed to initialize database");
            app.manage(DbState {
                conn: Arc::new(Mutex::new(conn)),
            });

            // Start networking in async runtime
            let db_conn = app.state::<DbState>().conn.clone();
            tauri::async_runtime::spawn(async move {
                let port = transfer::start_listener(handle.clone()).await.expect("failed to start transfer listener");
                
                let display_name = {
                    if let Ok(conn) = db_conn.lock() {
                        conn.query_row(
                            "SELECT value FROM settings WHERE key = 'displayName'",
                            [],
                            |r| r.get::<_, String>(0)
                        ).ok()
                    } else { None }
                }.unwrap_or_else(|| whoami::username().unwrap_or_else(|_| "Unknown".to_string()));

                mdns::start_discovery(handle, port, display_name).expect("failed to start mDNS discovery");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            discover_peers, 
            send_file,
            respond_to_transfer,
            cancel_transfer,
            get_history,
            clear_history,
            get_settings,
            save_settings,
            get_file_meta,
            direct_connect
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            println!("Application exiting, cleaning up...");
            mdns::shutdown_discovery(handle);
        }
    });
}
