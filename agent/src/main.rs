use crate::error::Result;
use clap::Parser;
use rand::{thread_rng, Rng};
use std::sync::{Arc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

mod capture;
mod cli;
mod error;
mod models;
mod network;
mod sys;

use cli::Args;
use models::ControlEvent;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();

    let machine_id = args.machine_id.clone().unwrap_or_else(|| {
        local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| {
                machine_uid::get().unwrap_or_else(|_| "unknown-machine".to_string())
            })
    });
        
    let mut rng = thread_rng();
    let password_str = args.password.clone()
        .unwrap_or_else(|| {
            rng.gen_range(100000..999999).to_string()
        });

    info!("========================================");
    info!("   SELFCONTROL AGENT v1.1");
    info!("   MACHINE ID: {}", machine_id);
    info!("   PASSWORD:   {}", password_str);
    info!("   MODE:       FULL ACCESS (Integrated Service)");
    info!("========================================");

    let is_streaming = Arc::new(Mutex::new(false));
    let display_index = Arc::new(Mutex::new(0usize));
    
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
    let (data_tx, data_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(50);
    let (response_tx, response_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(10);

    // Audio Capture Task
    let audio_tx = data_tx.clone();
    std::thread::spawn(move || {
        if let Err(e) = capture::audio::start_audio_capture(audio_tx) {
            tracing::warn!(error = %e, "Audio capture failed to start");
        }
    });

    // Video Capture Task
    let is_streaming_cap = is_streaming.clone();
    let display_index_cap = display_index.clone();
    let frame_tx = data_tx.clone();
    std::thread::spawn(move || {
        capture::video::start_video_capture(is_streaming_cap, display_index_cap, frame_tx);
    });

    // Input Control Handler Task
    let is_streaming_ctrl = is_streaming.clone();
    let display_index_ctrl = display_index.clone();
    let response_tx_ctrl = response_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                ControlEvent::StartCapture => {
                    info!("[SIGNAL] Starting Capture loop");
                    let mut s = is_streaming_ctrl.lock().unwrap();
                    *s = true;

                    // Send display metadata whenever a client joins
                    let res_tx = response_tx_ctrl.clone();
                    tokio::spawn(async move {
                        if let Ok(displays) = scrap::Display::all() {
                            let metadata = serde_json::json!({
                                "type": "metadata",
                                "displays": displays.iter().enumerate().map(|(i, d)| {
                                    serde_json::json!({
                                        "index": i,
                                        "width": d.width(),
                                        "height": d.height(),
                                        "is_primary": i == 0
                                    })
                                }).collect::<Vec<_>>()
                            });
                            let _ = res_tx.send(metadata).await;
                        }
                    });
                }
                ControlEvent::StopCapture => {
                    info!("[SIGNAL] Stopping Capture loop (Idle mode)");
                    let mut s = is_streaming_ctrl.lock().unwrap();
                    *s = false;
                }
                ControlEvent::SwitchDisplay { index } => {
                    info!(index = index, "[SIGNAL] Switching to display");
                    let mut idx = display_index_ctrl.lock().unwrap();
                    *idx = index;
                }
                ControlEvent::MouseMove { x, y } => {
                    let d_idx = display_index_ctrl.clone();
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();

                        let idx = { *d_idx.lock().unwrap() };
                        if let Ok(displays) = scrap::Display::all() {
                            if let Some(display) = displays.get(idx).or_else(|| displays.first()) {
                                use enigo::MouseControllable;
                                let mut enigo = enigo::Enigo::new();
                                let lx = x * display.logical_width() as f32 + display.origin_x() as f32;
                                let ly = y * display.logical_height() as f32 + display.origin_y() as f32;
                                enigo.mouse_move_to(lx as i32, ly as i32);
                            }
                        }
                    }).await.ok();
                }
                ControlEvent::MouseDown { button } => {
                    let btn = if button == "right" { enigo::MouseButton::Right } else { enigo::MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();
                        use enigo::MouseControllable;
                        let mut enigo = enigo::Enigo::new();
                        enigo.mouse_down(btn);
                    }).await.ok();
                }
                ControlEvent::MouseUp { button } => {
                    let btn = if button == "right" { enigo::MouseButton::Right } else { enigo::MouseButton::Left };
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();
                        use enigo::MouseControllable;
                        let mut enigo = enigo::Enigo::new();
                        enigo.mouse_up(btn);
                    }).await.ok();
                }
                ControlEvent::KeyDown { key } => {
                    let key_c = key.clone();
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();

                        use enigo::KeyboardControllable;
                        let mut enigo = enigo::Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_down(k);
                        }
                    }).await.ok();

                    if key_c.to_lowercase() == "c" {
                        let res_tx = response_tx_ctrl.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            #[cfg(all(target_os = "windows", feature = "windows_service"))]
                            let _desktop_guard = sys::windows_service::AutoDesktop::new();

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
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();

                        use enigo::KeyboardControllable;
                        let mut enigo = enigo::Enigo::new();
                        if let Some(k) = parse_key(&key) {
                            enigo.key_up(k);
                        }
                    }).await.ok();
                }
                ControlEvent::PasteText { text } => {
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();

                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(text);
                        }

                        // Trigger Paste via keyboard
                        use enigo::{Key, KeyboardControllable};
                        let mut enigo = enigo::Enigo::new();
                        
                        #[cfg(target_os = "macos")]
                        let modifier = Key::Meta;
                        #[cfg(not(target_os = "macos"))]
                        let modifier = Key::Control;

                        enigo.key_down(modifier);
                        enigo.key_click(Key::Layout('v'));
                        enigo.key_up(modifier);
                    }).await.ok();
                }
                ControlEvent::ResolutionUpdate { width, height } => {
                    let d_idx = display_index_ctrl.clone();
                    tokio::task::spawn_blocking(move || {
                        let idx = { *d_idx.lock().unwrap() };
                        info!(width = width, height = height, index = idx, "[SIGNAL] Updating Resolution");
                        if let Err(e) = set_resolution(idx, width, height) {
                            tracing::error!(error = %e, "Failed to update resolution");
                        }
                    }).await.ok();
                }
                ControlEvent::MouseWheel { delta_x, delta_y } => {
                    tokio::task::spawn_blocking(move || {
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = sys::windows_service::AutoDesktop::new();
                        use enigo::MouseControllable;
                        let mut enigo = enigo::Enigo::new();
                        if delta_x != 0 {
                            enigo.mouse_scroll_x(delta_x);
                        }
                        if delta_y != 0 {
                            enigo.mouse_scroll_y(delta_y);
                        }
                    }).await.ok();
                }
            }
        }
    });

    // Start WebSocket Network Loop
    network::ws::start_connection_loop(
        args.server,
        args.port,
        machine_id,
        password_str,
        is_streaming,
        display_index,
        event_tx,
        data_rx,
        response_rx,
    ).await?;

    Ok(())
}

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

fn set_resolution(display_index: usize, width: usize, height: usize) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        tracing::info!("Resolution switch requested for macOS display {} to {}x{} (Implementation pending)", display_index, width, height);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, ChangeDisplaySettingsExW, DEVMODEW, ENUM_CURRENT_SETTINGS, CDS_UPDATEREGISTRY, DISP_CHANGE_SUCCESSFUL};

        unsafe {
            let mut dev_mode: DEVMODEW = std::mem::zeroed();
            dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

            if EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut dev_mode).as_bool() {
                dev_mode.dmPelsWidth = width as u32;
                dev_mode.dmPelsHeight = height as u32;
                dev_mode.dmFields = windows::Win32::Graphics::Gdi::DM_PELSWIDTH | windows::Win32::Graphics::Gdi::DM_PELSHEIGHT;

                let result = ChangeDisplaySettingsExW(None, Some(&dev_mode), None, CDS_UPDATEREGISTRY, None);
                if result == DISP_CHANGE_SUCCESSFUL {
                    tracing::info!("Resolution changed successfully to {}x{}", width, height);
                    Ok(())
                } else {
                    anyhow::bail!("Failed to change resolution: {:?}", result)
                }
            } else {
                anyhow::bail!("Failed to enum display settings")
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        anyhow::bail!("Resolution switching not supported on this platform")
    }
}
