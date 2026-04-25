use rusqlite::{params, Connection};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use crate::transfer::TransferRequest;

pub fn init_db(app: &AppHandle) -> Result<Connection, rusqlite::Error> {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&app_dir).unwrap();
    let db_path = app_dir.join("dropbridge.db");
    
    let conn = Connection::open(db_path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS peers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            ip TEXT NOT NULL,
            port INTEGER NOT NULL,
            last_seen INTEGER NOT NULL
        )",
        [],
    )?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS transfers (
            id TEXT PRIMARY KEY,
            direction TEXT NOT NULL,
            peer_name TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            status TEXT NOT NULL
        )",
        [],
    )?;

    // Migration: Add timestamp if it doesn't exist
    // Wrapped in a scope so 'stmt' and 'rows' are dropped before returning 'conn'
    {
        let mut stmt = conn.prepare("PRAGMA table_info(transfers)")?;
        let mut rows = stmt.query([])?;
        let mut has_timestamp = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == "timestamp" {
                has_timestamp = true;
                break;
            }
        }

        if !has_timestamp {
            conn.execute("ALTER TABLE transfers ADD COLUMN timestamp INTEGER NOT NULL DEFAULT 0", [])?;
        }
    }
    
    Ok(conn)
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
