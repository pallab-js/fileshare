use rusqlite::{params, Connection};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use crate::transfer::TransferRequest;

pub fn init_db(app: &AppHandle) -> Result<Connection, rusqlite::Error> {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&app_dir).unwrap();
    let db_path = app_dir.join("dropbridge.db");
    
    let conn = Connection::open(db_path)?;
    
    // Enable WAL mode
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
            CREATE INDEX IF NOT EXISTS idx_transfers_timestamp ON transfers(timestamp DESC);
            PRAGMA user_version = 1;
        ")?;
    }
    
    // Purge peers older than 24h
    conn.execute(
        "DELETE FROM peers WHERE last_seen < ?1",
        params![chrono::Utc::now().timestamp() - 86400],
    )?;
    
    Ok(conn)
}

pub fn upsert_peer(
    conn: &Connection,
    peer: &crate::mdns::Peer,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO peers (id, name, ip, port, last_seen, device_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![peer.id, peer.name, peer.ip, peer.port, peer.last_seen, peer.device_type],
    )?;
    Ok(())
}

pub fn log_transfer(
    conn: &Connection,
    direction: &str,
    request: &TransferRequest,
    status: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO transfers (id, direction, peer_name, file_name, file_size, status, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            request.id,
            direction,
            request.sender,
            request.file_name,
            request.file_size,
            status,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}
