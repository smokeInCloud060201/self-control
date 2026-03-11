use crc32fast::Hasher;
use image::{codecs::jpeg::JpegEncoder, RgbImage};
use scrap::{Capturer, Display};
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, warn};

pub fn start_video_capture(
    is_streaming: Arc<Mutex<bool>>,
    display_index: Arc<Mutex<usize>>,
    frame_tx: Sender<Vec<u8>>,
) {
    let mut capturer_opt: Option<Capturer> = None;
    let mut last_status = std::time::Instant::now();
    let mut frame_sent = 0;
    let mut last_frame_hash: u32 = 0;
    let mut current_display_idx = 0;

    loop {
        #[cfg(all(target_os = "windows", feature = "windows_service"))]
        let _desktop_guard = crate::sys::windows_service::AutoDesktop::new();

        // 0. Check if display index changed
        let target_display_idx = { *display_index.lock().unwrap() };
        if target_display_idx != current_display_idx {
            info!(new_index = target_display_idx, "Display switch requested");
            capturer_opt = None;
            current_display_idx = target_display_idx;
            last_frame_hash = 0; // Force full frame on switch
        }

        let streaming = { *is_streaming.lock().unwrap() };
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
                        let mut payload = vec![0x01]; // Video Type
                        payload.extend_from_slice(&buffer);
                        if let Err(_) = frame_tx.blocking_send(payload) {
                            break; // Receiver dropped
                        }
                        frame_sent += 1;
                    }
                }
                
                if last_status.elapsed().as_secs() >= 5 {
                    #[cfg(target_os = "macos")]
                    let login_window = crate::sys::macos_session::is_login_window();
                    #[cfg(not(target_os = "macos"))]
                    let login_window = false;

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
}
