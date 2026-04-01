use crate::app_state::AppState;
use crate::models::ControlEvent;
use crate::error::Result;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::info;

pub async fn start_handler(
    mut event_rx: Receiver<ControlEvent>,
    state: AppState,
    response_tx: Sender<serde_json::Value>,
) {
    let is_streaming_ctrl = state.is_streaming.clone();
    let display_index_ctrl = state.display_index.clone();
    let response_tx_ctrl = response_tx.clone();

    while let Some(event) = event_rx.recv().await {
        match event {
            ControlEvent::StartCapture => {
                info!("[SIGNAL] Starting Capture loop");
                let mut s = is_streaming_ctrl.lock().unwrap();
                *s = true;

                let res_tx = response_tx_ctrl.clone();
                tokio::spawn(async move {
                    let metadata = if let Ok(displays) = scrap::Display::all() {
                        Some(serde_json::json!({
                            "type": "metadata",
                            "displays": displays.iter().enumerate().map(|(i, d)| {
                                serde_json::json!({
                                    "index": i,
                                    "width": d.width(),
                                    "height": d.height(),
                                    "is_primary": i == 0
                                })
                            }).collect::<Vec<_>>()
                        }))
                    } else {
                        None
                    };

                    if let Some(metadata) = metadata {
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
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

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
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();
                    use enigo::MouseControllable;
                    let mut enigo = enigo::Enigo::new();
                    enigo.mouse_down(btn);
                }).await.ok();
            }
            ControlEvent::MouseUp { button } => {
                let btn = if button == "right" { enigo::MouseButton::Right } else { enigo::MouseButton::Left };
                tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "windows")]
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();
                    use enigo::MouseControllable;
                    let mut enigo = enigo::Enigo::new();
                    enigo.mouse_up(btn);
                }).await.ok();
            }
            ControlEvent::KeyDown { key } => {
                let key_c = key.clone();
                tokio::task::spawn_blocking(move || {
                    #[cfg(target_os = "windows")]
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

                    use enigo::KeyboardControllable;
                    let mut enigo = enigo::Enigo::new();
                    if let Some(k) = parse_key(&key_c) {
                        enigo.key_down(k);
                    }
                }).await.ok();

                if key.to_lowercase() == "c" {
                    let res_tx = response_tx_ctrl.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        #[cfg(target_os = "windows")]
                        let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

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
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

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
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

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
                    let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();
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

fn set_resolution(_display_index: usize, width: usize, height: usize) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        tracing::info!("Resolution switch requested for macOS display {} to {}x{} (Implementation pending)", _display_index, width, height);
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        unsafe {
            let mut dev_mode: ::windows::Win32::Graphics::Gdi::DEVMODEW = std::mem::zeroed();
            dev_mode.dmSize = std::mem::size_of::<::windows::Win32::Graphics::Gdi::DEVMODEW>() as u16;

            if ::windows::Win32::Graphics::Gdi::EnumDisplaySettingsW(None, ::windows::Win32::Graphics::Gdi::ENUM_CURRENT_SETTINGS, &mut dev_mode).as_bool() {
                dev_mode.dmPelsWidth = width as u32;
                dev_mode.dmPelsHeight = height as u32;
                dev_mode.dmFields = ::windows::Win32::Graphics::Gdi::DM_PELSWIDTH | ::windows::Win32::Graphics::Gdi::DM_PELSHEIGHT;

                let result = ::windows::Win32::Graphics::Gdi::ChangeDisplaySettingsExW(None, Some(&dev_mode), None, ::windows::Win32::Graphics::Gdi::CDS_UPDATEREGISTRY, None);
                if result == ::windows::Win32::Graphics::Gdi::DISP_CHANGE_SUCCESSFUL {
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
