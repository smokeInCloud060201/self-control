use crate::error::Result;
use crate::models::ControlEvent;
use futures_util::{SinkExt, StreamExt};

use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use tracing::{debug, error, info, warn};
use scrap::Display;
use std::sync::Arc;

use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidateInit, RTCIceCandidate};

use crate::app_state::AppState;

pub async fn start_connection_loop(
    server: String,
    port: u16,
    state: AppState,
    event_tx: tokio::sync::mpsc::Sender<ControlEvent>,
    mut data_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    mut response_rx: tokio::sync::mpsc::Receiver<serde_json::Value>,
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
        let current_pwd = { state.password_shared.lock().unwrap().clone() };
        let proxy_url = format!("{}://{}:{}/agent/{}/{}", scheme, server_clean, port, state.machine_id, current_pwd);
        
        debug!(url = %proxy_url, "Connecting to proxy");
        let ws_stream = match connect_async_with_config(&proxy_url, Some(config), true).await {
            Ok((s, _)) => s,
            Err(e) => {
                error!(error = %e, "Proxy connection failed to {}:{}, retrying in 3s", server_clean, port);
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        { *state.status.lock().unwrap() = "Connected".to_string(); }
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

        // WebRTC Initialization
        let (signaling_tx, mut signaling_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(50);
        let signaling_tx_clone_for_ice = signaling_tx.clone();
        
        // This is safe to run since webrtc is memory safe
        let (pc, mut video_dc_rx) = crate::network::webrtc::setup_webrtc(
            event_tx.clone()
        ).await?;

        let pc_clone = pc.clone();
        pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            let sig_tx = signaling_tx_clone_for_ice.clone();
            Box::pin(async move {
                if let Some(candidate) = c {
                    if let Ok(json) = candidate.to_json() {
                        let msg = serde_json::json!({
                            "type": "ice_candidate",
                            "candidate": json
                        });
                        let _ = sig_tx.send(msg).await;
                    }
                }
            })
        }));

        // Relay Control Events from Proxy (SDP + ICE)
        let signaling_tx_for_ws = signaling_tx.clone();
        let mut control_task = tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                if let Ok(Message::Text(text)) = msg {
                    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                    let msg_type = json["type"].as_str().unwrap_or("");
                    
                    if msg_type == "offer" {
                        if let Ok(sdp) = serde_json::from_value::<RTCSessionDescription>(json["sdp"].clone()) {
                            if let Err(e) = pc_clone.set_remote_description(sdp).await {
                                error!("Failed to set remote description: {}", e);
                                continue;
                            }
                            if let Ok(answer) = pc_clone.create_answer(None).await {
                                if let Ok(_) = pc_clone.set_local_description(answer.clone()).await {
                                    let ans_msg = serde_json::json!({
                                        "type": "answer",
                                        "sdp": answer
                                    });
                                    let _ = signaling_tx_for_ws.send(ans_msg).await;
                                }
                            }
                        }
                    } else if msg_type == "ice_candidate" {
                        if let Ok(candidate) = serde_json::from_value::<RTCIceCandidateInit>(json["candidate"].clone()) {
                            let _ = pc_clone.add_ice_candidate(candidate).await;
                        }
                    } else if let Ok(event) = serde_json::from_str::<ControlEvent>(&text) {
                        let _ = event_tx_clone.send(event).await; // Legacy fallback
                    }
                }
            }
        });

        // Loop to forward JPEG Frames to DataChannel
        let mut active_video_dc = None;

        loop {
            tokio::select! {
                Some(dc) = video_dc_rx.recv() => {
                    info!("Agent registered Video RTCDataChannel");
                    active_video_dc = Some(dc);
                }
                Some(payload) = data_rx.recv() => {
                    if let Some(dc) = &active_video_dc {
                        let bytes = bytes::Bytes::from(payload);
                        if let Err(e) = dc.send(&bytes).await {
                            debug!(error = %e, "WebRTC DataChannel frame send dropped");
                        }
                    }
                }
                Some(msg) = signaling_rx.recv() => {
                    let text = msg.to_string();
                    if let Err(e) = ws_sender.send(Message::Text(text)).await {
                        debug!(error = %e, "Signaling TCP send failed");
                        break;
                    }
                }
                _ = &mut control_task => {
                    break;
                }
            }
        }
        
        control_task.abort();
        { *state.is_streaming.lock().unwrap() = false; }
        { *state.status.lock().unwrap() = "Disconnected".to_string(); }
        warn!("Connection lost, re-establishing in 2s...");
        sleep(Duration::from_secs(2)).await;
    }
}
