use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use scrap::{Display, Capturer};
use std::io::ErrorKind;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use image::{RgbImage, codecs::jpeg::JpegEncoder};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use enigo::{Enigo, MouseButton, MouseControllable};
use std::sync::{Arc, Mutex};
use rand::{thread_rng, Rng};
use tracing::{info, warn, error, debug, Level};
use tracing_subscriber::EnvFilter;
use thiserror::Error;
mod macos_session;
mod windows_service;

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
    KeyDown { key: String },
    KeyUp { key: String },
    #[serde(rename = "switch_display")]
    SwitchDisplay { index: usize },
    #[serde(rename = "paste_text")]
    PasteText { text: String },
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost", env = "PROXY_SERVER")]
    server: String,
    #[arg(short, long, default_value_t = 8080, env = "PROXY_PORT")]
    port: u16,
    #[arg(long)]
    service: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();
    
    let mut server = args.server.clone();
    let mut scheme = "ws";
    if server.contains("://") {
        if server.starts_with("https://") {
            scheme = "wss";
        }
        server = server.split("://").last().unwrap_or(&server).to_string();
    }
    // Remove any trailing slashes
    server = server.trim_end_matches('/').to_string();

    // Use local IP as machine ID as suggested by user
    let machine_id = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| {
            machine_uid::get().unwrap_or_else(|_| "unknown-machine".to_string())
        });
    let mut rng = thread_rng();
    let password_str = if args.service {
        "admin123".to_string() // Static password for service mode for now
    } else {
        rng.gen_range(100000..999999).to_string()
    };

    info!("========================================");
    info!("   RUST REMOTE AGENT v2.5");
    info!("   MACHINE ID: {}", machine_id);
    info!("   PASSWORD:   {}", password_str);
    if args.service {
        info!("   MODE:       SYSTEM SERVICE");
    }
    info!("========================================");

    let is_streaming = Arc::new(Mutex::new(false));
    let display_index = Arc::new(Mutex::new(0usize));
    let display_index_cap = display_index.clone();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
    let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);
    let (response_tx, mut response_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(10);
    
    // Capture Thread (dedicated OS thread to avoid Tokio runtime conflicts)
    let is_streaming_cap = is_streaming.clone();
    std::thread::spawn(move || {
        let mut capturer_opt: Option<Capturer> = None;
        let mut last_status = std::time::Instant::now();
        let mut frame_sent = 0;
        let mut last_frame_hash: u32 = 0;

        let mut current_display_idx = 0;

        loop {
            #[cfg(all(target_os = "windows", feature = "windows_service"))]
            let _desktop_guard = windows_service::AutoDesktop::new();

            // 0. Check if display index changed
            let target_display_idx = { *display_index_cap.lock().unwrap() };
            if target_display_idx != current_display_idx {
                info!(new_index = target_display_idx, "Display switch requested");
                capturer_opt = None;
                current_display_idx = target_display_idx;
                last_frame_hash = 0; // Force full frame on switch
            }

            let streaming = { *is_streaming_cap.lock().unwrap() };
            if !streaming {
                capturer_opt = None;
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }

            if capturer_opt.is_none() {
                match Display::all() {
                    Ok(displays) => {
                        if let Some(display) = displays.get(current_display_idx).or_else(|| displays.first()) {
                            match Capturer::new((*display).clone()) {
                                Ok(c) => {
                                    info!(width = c.width(), height = c.height(), index = current_display_idx, "Capturer initialized");
                                    capturer_opt = Some(c);
                                }
                                Err(e) => {
                                    warn!(error = %e, "Capturer init failed, retrying");
                                    std::thread::sleep(Duration::from_millis(500));
                                    continue;
                                }
                            }
                        } else {
                            warn!("No displays found");
                            std::thread::sleep(Duration::from_millis(1000));
                            continue;
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Display enumeration failed");
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }

            let capturer = capturer_opt.as_mut().unwrap();
            let (width, height) = (capturer.width(), capturer.height());

            match capturer.frame() {
                Ok(frame) => {
                    let expected = width * height * 4;
                    if frame.len() < expected { continue; }

                    // 1. Calculate Hash to detect changes
                    let mut hasher = Hasher::new();
                    hasher.update(&frame[..expected]);
                    let current_hash = hasher.finalize();

                    if current_hash == last_frame_hash {
                        // Skip frame if identical
                        std::thread::sleep(Duration::from_millis(10)); 
                        continue;
                    }
                    last_frame_hash = current_hash;

                    let mut buffer = Vec::new();
                    // 2. Use JPEG with tuned quality
                    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 40);
                    
                    let mut rgb_data = vec![0u8; width * height * 3];
                    for (i, chunk) in frame[..expected].chunks_exact(4).enumerate() {
                        rgb_data[i * 3] = chunk[2];
                        rgb_data[i * 3 + 1] = chunk[1];
                        rgb_data[i * 3 + 2] = chunk[0];
                    }

                    if let Some(img) = RgbImage::from_raw(width as u32, height as u32, rgb_data) {
                        if let Ok(_) = encoder.encode_image(&img) {
                            if let Err(_) = frame_tx.blocking_send(buffer) {
                                break; // Receiver dropped
                            }
                            frame_sent += 1;
                        }
                    }
                    
                    if last_status.elapsed().as_secs() >= 5 {
                        let login_window = macos_session::is_login_window();
                        if login_window {
                            info!("[STATUS] Uplink: {} fps (LOGIN WINDOW DETECTED)", frame_sent / 5);
                        } else {
                            info!("[STATUS] Uplink: {} fps", frame_sent / 5);
                        }
                        frame_sent = 0;
                        last_status = std::time::Instant::now();
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(16));
                }
                Err(e) => {
                    debug!(error = %e, "Capture error, resetting capturer");
                    capturer_opt = None; 
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    });

    // Input Handler Task
    let is_streaming_ctrl = is_streaming.clone();
    let response_tx_ctrl = response_tx.clone();
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
                    let d_idx = display_index.clone();
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        let idx = { *d_idx.lock().unwrap() };
                        if let Ok(displays) = Display::all() {
                            if let Some(display) = displays.get(idx).or_else(|| displays.first()) {
                                let mut enigo = Enigo::new();
                                let lx = x * display.logical_width() as f32 + display.origin_x() as f32;
                                let ly = y * display.logical_height() as f32 + display.origin_y() as f32;
                                enigo.mouse_move_to(lx as i32, ly as i32);
                            }
                        }
                    }).await.ok();
                }
                ControlEvent::MouseDown { button } => {
                    let btn = if button == "right" { MouseButton::Right } else { MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        let mut enigo = Enigo::new();
                        enigo.mouse_down(btn);
                    }).await.ok();
                }
                ControlEvent::MouseUp { button } => {
                    let btn = if button == "right" { MouseButton::Right } else { MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        let mut enigo = Enigo::new();
                        enigo.mouse_up(btn);
                    }).await.ok();
                }
                ControlEvent::KeyDown { key } => {
                    let key_c = key.clone();
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        use enigo::KeyboardControllable;
                        let mut enigo = Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_down(k);
                        }
                    }).await.ok();

                    // Shortcut Detection: Detect Copy (Cmd+C on Mac, Ctrl+C otherwise)
                    // We trigger a "Check Clipboard" on every key down if it's 'c'.
                    if key_c.to_lowercase() == "c" {
                        let res_tx = response_tx_ctrl.clone();
                        tokio::spawn(async move {
                            // Wait for remote app to process the copy command
                            sleep(Duration::from_millis(200)).await;
                            
                            #[cfg(all(target_os = "windows", feature = "windows_service"))]
                            let _desktop_guard = windows_service::AutoDesktop::new();

                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    let msg = serde_json::json!({
                                        "type": "clipboard_sync",
                                        "text": text
                                    });
                                    let _ = res_tx.send(msg).await;
                                }
                            }
                        });
                    }
                }
                ControlEvent::KeyUp { key } => {
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        use enigo::KeyboardControllable;
                        let mut enigo = Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_up(k);
                        }
                    }).await.ok();
                }
                ControlEvent::SwitchDisplay { index } => {
                    info!(index = index, "[SIGNAL] Switching to display");
                    let mut idx = display_index.lock().unwrap();
                    *idx = index;
                }
                ControlEvent::PasteText { text } => {
                    tokio::task::spawn_blocking(move || {
                        #[cfg(all(target_os = "windows", feature = "windows_service"))]
                        let _desktop_guard = windows_service::AutoDesktop::new();

                        // 1. Set clipboard
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(text);
                        }

                        // 2. Trigger Paste (Cmd+V on Mac, Ctrl+V otherwise)
                        use enigo::{Key, KeyboardControllable};
                        let mut enigo = Enigo::new();
                        
                        #[cfg(target_os = "macos")]
                        let modifier = Key::Meta;
                        #[cfg(not(target_os = "macos"))]
                        let modifier = Key::Control;

                        enigo.key_down(modifier);
                        enigo.key_click(Key::Layout('v'));
                        enigo.key_up(modifier);
                    }).await.ok();
                }
            }
        }
    });

fn parse_key(key: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match key.to_lowercase().as_str() {
        "enter" | "return" => Some(Key::Return),
        "backspace" => Some(Key::Backspace),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "escape" | "esc" => Some(Key::Escape),
        "control" | "ctrl" => Some(Key::Control),
        "shift" => Some(Key::Shift),
        "alt" | "option" => Some(Key::Alt),
        "meta" | "command" | "win" | "super" => Some(Key::Meta),
        "up" | "arrowup" => Some(Key::UpArrow),
        "down" | "arrowdown" => Some(Key::DownArrow),
        "left" | "arrowleft" => Some(Key::LeftArrow),
        "right" | "arrowright" => Some(Key::RightArrow),
        k if k.len() == 1 => Some(Key::Layout(k.chars().next().unwrap())),
        _ => None,
    }
}

    let proxy_url = format!("{}://{}:{}/agent/{}/{}", scheme, server, args.port, machine_id, password_str);
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
        info!("Connected to proxy {}:{}", args.server, args.port);

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

        // Relay Frames & Responses To Proxy
        loop {
            tokio::select! {
                Some(buffer) = frame_rx.recv() => {
                    if let Err(e) = ws_sender.send(Message::Binary(buffer)).await {
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
        warn!("Connection lost, re-establishing in 2s...");
        sleep(Duration::from_secs(2)).await;
    }
}
