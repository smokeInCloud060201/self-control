use crate::error::Result;
use crate::models::ControlEvent;
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use tracing::{debug, error, info, warn};
use scrap::Display;

pub async fn start_connection_loop(
    server: String,
    port: u16,
    machine_id: String,
    password: Arc<Mutex<String>>,
    is_streaming: Arc<Mutex<bool>>,
    _display_index: Arc<Mutex<usize>>,
    event_tx: tokio::sync::mpsc::Sender<ControlEvent>,
    mut data_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    mut response_rx: tokio::sync::mpsc::Receiver<serde_json::Value>,
    status: Arc<Mutex<String>>,
) -> Result<()> {
    let mut scheme = "ws";
    let mut server_clean = server.clone();
    if server_clean.contains("://") {
        if server_clean.starts_with("https://") {
            scheme = "wss";
        }
        server_clean = server_clean.split("://").last().unwrap_or(&server_clean).to_string();
    }
    server_clean = server_clean.trim_end_matches('/').to_string();

    let config = WebSocketConfig {
        max_message_size: Some(128 * 1024 * 1024),
        max_frame_size: Some(32 * 1024 * 1024),
        ..Default::default()
    };
    
    loop {
        let current_pwd = { password.lock().unwrap().clone() };
        let proxy_url = format!("{}://{}:{}/agent/{}/{}", scheme, server_clean, port, machine_id, current_pwd);
        
        debug!(url = %proxy_url, "Connecting to proxy");
        let ws_stream = match connect_async_with_config(&proxy_url, Some(config), true).await {
            Ok((s, _)) => s,
            Err(e) => {
                error!(error = %e, "Proxy connection failed to {}:{}, retrying in 3s", server_clean, port);
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        { *status.lock().unwrap() = "Connected".to_string(); }
        info!("Connected to proxy {}:{}", server_clean, port);

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let event_tx_clone = event_tx.clone();

        // 0. Send Display Metadata
        if let Ok(displays) = Display::all() {
            let metadata = serde_json::json!({
                "type": "metadata",
                "displays": displays.iter().enumerate().map(|(i, d)| {
                    serde_json::json!({
                        "index": i,
                        "width": d.width(),
                        "height": d.height(),
                        "is_primary": i == 0 // Simplification
                    })
                }).collect::<Vec<_>>()
            });
            if let Ok(text) = serde_json::from_value::<serde_json::Value>(metadata) {
                let _ = ws_sender.send(Message::Text(text.to_string())).await;
            }
        }

        // Relay Control Events from Proxy
        let mut control_task = tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(event) = serde_json::from_str::<ControlEvent>(&text) {
                            let _ = event_tx_clone.send(event).await;
                        }
                    }
                    Ok(Message::Ping(_)) => {}
                    Ok(_) => {},
                    Err(e) => {
                        debug!(error = %e, "Control stream error");
                        break;
                    }
                }
            }
        });

        // Relay Data & Responses To Proxy
        loop {
            tokio::select! {
                Some(payload) = data_rx.recv() => {
                    if let Err(e) = ws_sender.send(Message::Binary(payload)).await {
                        debug!(error = %e, "Relay send failed");
                        break;
                    }
                }
                Some(response) = response_rx.recv() => {
                    if let Ok(text) = serde_json::to_string(&response) {
                        if let Err(e) = ws_sender.send(Message::Text(text)).await {
                            debug!(error = %e, "Response relay failed");
                            break;
                        }
                    }
                }
                _ = &mut control_task => {
                    break;
                }
            }
        }
        
        control_task.abort();
        { *is_streaming.lock().unwrap() = false; }
        { *status.lock().unwrap() = "Disconnected".to_string(); }
        warn!("Connection lost, re-establishing in 2s...");
        sleep(Duration::from_secs(2)).await;
    }
}
