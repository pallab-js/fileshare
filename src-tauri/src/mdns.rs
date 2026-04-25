use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub last_seen: i64,
}

pub struct DiscoveryState {
    pub peers: Arc<Mutex<HashMap<String, Peer>>>,
}

pub fn start_discovery(app: AppHandle, transfer_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_dropbridge._tcp.local.";
    let devicename = whoami::devicename().unwrap_or_else(|_| "UnknownMac".to_string());
    let my_name = format!("{}._dropbridge._tcp.local.", devicename);
    
    // Register our service
    let my_ip = local_ip_address::local_ip()?.to_string();
    let username = whoami::username().unwrap_or_else(|_| "UnknownUser".to_string());
    let properties = [("name", username)];
    
    let my_service = ServiceInfo::new(
        service_type,
        &my_name,
        &format!("{}.local.", devicename),
        &my_ip,
        transfer_port,
        &properties[..],
    )?;
    
    mdns.register(my_service)?;
    
    // Browse for others
    let receiver = mdns.browse(service_type)?;
    
    let state = app.state::<DiscoveryState>();
    let peers_clone = state.peers.clone();
    
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let peer_id = info.get_fullname().to_string();
                    let ip = info.get_addresses().iter().next().map(|ip| ip.to_string()).unwrap_or_default();
                    
                    let name = match info.get_property_val("name") {
                        Some(Some(val)) => String::from_utf8_lossy(val).to_string(),
                        _ => "Unknown".to_string(),
                    };
                    
                    let peer = Peer {
                        id: peer_id.clone(),
                        name,
                        ip,
                        port: info.get_port(),
                        last_seen: chrono::Utc::now().timestamp(),
                    };
                    
                    let mut peers = peers_clone.lock().unwrap();
                    peers.insert(peer_id.clone(), peer.clone());
                    
                    let _ = app.emit("peer_discovered", peer);
                }
                ServiceEvent::ServiceRemoved(_type, name) => {
                    let mut peers = peers_clone.lock().unwrap();
                    peers.remove(&name);
                    let _ = app.emit("peer_removed", name);
                }
                _ => {}
            }
        }
    });
    
    Ok(())
}
