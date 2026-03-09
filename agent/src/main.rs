use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use scrap::{Display, Capturer};
use std::io::ErrorKind;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use image::{RgbImage, codecs::jpeg::JpegEncoder};
use serde::{Deserialize, Serialize};
use enigo::{Enigo, MouseButton, MouseControllable};
use std::sync::{Arc, Mutex};
use rand::{thread_rng, Rng};
use tracing::{info, warn, error, debug, instrument, Level};
use tracing_subscriber::EnvFilter;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Capture failed: {0}")]
    Capture(String),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Display not found")]
    DisplayNotFound,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ControlEvent {
    #[serde(rename = "start_capture")]
    StartCapture,
    #[serde(rename = "stop_capture")]
    StopCapture,
    MouseMove { x: f32, y: f32 },
    MouseDown { button: String },
    MouseUp { button: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let machine_id = machine_uid::get().unwrap_or_else(|_| "unknown-machine".to_string());
    let mut rng = thread_rng();
    let password: u32 = rng.gen_range(100000..999999);
    let password_str = password.to_string();

    info!("========================================");
    info!("   RUST REMOTE AGENT v2.5");
    info!("   MACHINE ID: {}", machine_id);
    info!("   PASSWORD:   {}", password_str);
    info!("========================================");

    let is_streaming = Arc::new(Mutex::new(false));
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
    
    // Input Handler Task
    let is_streaming_ctrl = is_streaming.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                ControlEvent::StartCapture => {
                    info!("[SIGNAL] Starting Capture loop");
                    let mut s = is_streaming_ctrl.lock().unwrap();
                    *s = true;
                }
                ControlEvent::StopCapture => {
                    info!("[SIGNAL] Stopping Capture loop (Idle mode)");
                    let mut s = is_streaming_ctrl.lock().unwrap();
                    *s = false;
                }
                ControlEvent::MouseMove { x, y } => {
                    tokio::task::spawn_blocking(move || {
                        if let Ok(display) = Display::primary() {
                            let mut enigo = Enigo::new();
                            enigo.mouse_move_to((x * display.width() as f32) as i32, (y * display.height() as f32) as i32);
                        }
                    }).await.ok();
                }
                ControlEvent::MouseDown { button } => {
                    let btn = if button == "right" { MouseButton::Right } else { MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        let mut enigo = Enigo::new();
                        enigo.mouse_down(btn);
                    }).await.ok();
                }
                ControlEvent::MouseUp { button } => {
                    let btn = if button == "right" { MouseButton::Right } else { MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        let mut enigo = Enigo::new();
                        enigo.mouse_up(btn);
                    }).await.ok();
                }
            }
        }
    });

    let proxy_url = format!("ws://localhost:8080/agent/{}/{}", machine_id, password_str);
    let config = WebSocketConfig {
        max_message_size: Some(128 * 1024 * 1024),
        max_frame_size: Some(32 * 1024 * 1024),
        ..Default::default()
    };
    
    loop {
        debug!(url = %proxy_url, "Connecting to proxy");
        let ws_stream = match connect_async_with_config(&proxy_url, Some(config), true).await {
            Ok((s, _)) => s,
            Err(e) => {
                error!(error = %e, "Proxy connection failed, retrying in 3s");
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        info!("Connected to proxy");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let event_tx_clone = event_tx.clone();

        // Relay Control Events from Proxy
        let control_task = tokio::spawn(async move {
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

        // Frame Capture loop
        let mut frame_sent = 0;
        let mut last_status = std::time::Instant::now();

        loop {
            if control_task.is_finished() { break; }

            let streaming = { *is_streaming.lock().unwrap() };
            if !streaming {
                sleep(Duration::from_millis(200)).await;
                continue;
            }

            // Lazy init capturer to handle display changes
            let display = match Display::primary() {
                Ok(d) => d,
                Err(e) => {
                    error!(error = %e, "Display link lost");
                    break;
                }
            };
            let mut capturer = match Capturer::new(display) {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Capturer init failed, retrying");
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            let (width, height) = (capturer.width(), capturer.height());

            match capturer.frame() {
                Ok(frame) => {
                    let expected = width * height * 4;
                    if frame.len() < expected { continue; }

                    let mut buffer = Vec::new();
                    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 50);
                    
                    let mut rgb_data = Vec::with_capacity(width * height * 3);
                    for chunk in frame[..expected].chunks_exact(4) {
                        rgb_data.push(chunk[2]);
                        rgb_data.push(chunk[1]);
                        rgb_data.push(chunk[0]);
                    }

                    if let Some(img) = RgbImage::from_raw(width as u32, height as u32, rgb_data) {
                        if let Ok(_) = encoder.encode_image(&img) {
                            if let Err(e) = ws_sender.send(Message::Binary(buffer)).await {
                                debug!(error = %e, "Relay send failed");
                                break;
                            }
                            frame_sent += 1;
                        }
                    }
                    
                    if last_status.elapsed().as_secs() >= 5 {
                        info!("[STATUS] Uplink: {} fps", frame_sent / 5);
                        frame_sent = 0;
                        last_status = std::time::Instant::now();
                    }
                    sleep(Duration::from_millis(50)).await; // ~20fps
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    sleep(Duration::from_millis(16)).await;
                    continue;
                }
                Err(e) => {
                    debug!(error = %e, "Capture skip");
                    continue;
                }
            }
        }
        
        control_task.abort();
        { *is_streaming.lock().unwrap() = false; }
        warn!("Connection lost, re-establishing in 2s...");
        sleep(Duration::from_secs(2)).await;
    }
}
