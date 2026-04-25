// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mdns;
mod transfer;
mod db;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Manager;
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
fn send_file(
    app: tauri::AppHandle,
    file_path: String,
    recipient_ip: String,
    port: u16,
) -> Result<String, String> {
    transfer::send_file(app, PathBuf::from(file_path), recipient_ip, port)
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
fn get_history(state: tauri::State<'_, DbState>) -> Result<Vec<TransferHistory>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, direction, peer_name, file_name, file_size, status, timestamp FROM transfers ORDER BY timestamp DESC")
        .map_err(|e| e.to_string())?;
    
    let history_iter = stmt.query_map([], |row| {
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
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM transfers", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, DbState>) -> Result<HashMap<String, String>, String> {
    let conn = state.conn.lock().unwrap();
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
}

#[tauri::command]
fn save_settings(state: tauri::State<'_, DbState>, settings: HashMap<String, String>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    for (key, value) in settings {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn claim_admin() -> Result<(), String> {
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(DiscoveryState {
            peers: Arc::new(Mutex::new(HashMap::new())),
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
            tauri::async_runtime::spawn(async move {
                let port = transfer::start_listener(handle.clone()).await.expect("failed to start transfer listener");
                mdns::start_discovery(handle, port).expect("failed to start mDNS discovery");
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
            claim_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
