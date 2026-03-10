use anyhow::Result;
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
use tracing::{info, warn, error, debug, Level};
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
    KeyDown { key: String },
    KeyUp { key: String },
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost", env = "PROXY_SERVER")]
    server: String,
    #[arg(short, long, default_value_t = 8080, env = "PROXY_PORT")]
    port: u16,
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
    let password: u32 = rng.gen_range(100000..999999);
    let password_str = password.to_string();

    info!("========================================");
    info!("   RUST REMOTE AGENT v2.5");
    info!("   MACHINE ID: {}", machine_id);
    info!("   PASSWORD:   {}", password_str);
    info!("========================================");

    let is_streaming = Arc::new(Mutex::new(false));
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
    let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(10);
    
    // Capture Thread (dedicated OS thread to avoid Tokio runtime conflicts)
    let is_streaming_cap = is_streaming.clone();
    std::thread::spawn(move || {
        let mut capturer_opt: Option<Capturer> = None;
        let mut last_status = std::time::Instant::now();
        let mut frame_sent = 0;

        loop {
            let streaming = { *is_streaming_cap.lock().unwrap() };
            if !streaming {
                capturer_opt = None;
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }

            if capturer_opt.is_none() {
                match Display::primary() {
                    Ok(display) => {
                        match Capturer::new(display) {
                            Ok(c) => {
                                info!(width = c.width(), height = c.height(), "Capturer initialized");
                                capturer_opt = Some(c);
                            }
                            Err(e) => {
                                warn!(error = %e, "Capturer init failed, retrying");
                                std::thread::sleep(Duration::from_millis(500));
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Display link lost, retrying");
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
                            if let Err(_) = frame_tx.blocking_send(buffer) {
                                break; // Receiver dropped
                            }
                            frame_sent += 1;
                        }
                    }
                    
                    if last_status.elapsed().as_secs() >= 5 {
                        info!("[STATUS] Uplink: {} fps", frame_sent / 5);
                        frame_sent = 0;
                        last_status = std::time::Instant::now();
                    }
                    std::thread::sleep(Duration::from_millis(50));
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
                            let lx = x * display.logical_width() as f32;
                            let ly = y * display.logical_height() as f32;
                            enigo.mouse_move_to(lx as i32, ly as i32);
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
                ControlEvent::KeyDown { key } => {
                    tokio::task::spawn_blocking(move || {
                        use enigo::KeyboardControllable;
                        let mut enigo = Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_down(k);
                        }
                    }).await.ok();
                }
                ControlEvent::KeyUp { key } => {
                    tokio::task::spawn_blocking(move || {
                        use enigo::KeyboardControllable;
                        let mut enigo = Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_up(k);
                        }
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

        // Relay Frames To Proxy
        loop {
            tokio::select! {
                Some(buffer) = frame_rx.recv() => {
                    if let Err(e) = ws_sender.send(Message::Binary(buffer)).await {
                        debug!(error = %e, "Relay send failed");
                        break;
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
