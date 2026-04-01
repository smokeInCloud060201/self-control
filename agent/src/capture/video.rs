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
        #[cfg(target_os = "windows")]
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
                let mut real_width = width;
                let mut real_height = height;

                let expected = width * height * 4;
                if frame.len() < expected { continue; }

                // Retina and display scaling check
                if frame.len() > expected {
                    // We assume it's scaled. Common scales are 2x (retina) 
                    let scale_squared = frame.len() / expected;
                    if scale_squared == 4 {
                        real_width = width * 2;
                        real_height = height * 2;
                    } else if scale_squared == 1 {
                        // Just padding?
                        // If it's padding, stride is frame.len() / height
                    } else {
                        // Best guess: square root of scale
                        let scale = (scale_squared as f64).sqrt() as usize;
                        if scale * scale == scale_squared {
                            real_width = width * scale;
                            real_height = height * scale;
                        } else {
                            // Can't guess, maybe irregular padding
                            tracing::warn!("Unpredictable frame scale. Length: {}, expected: {}", frame.len(), expected);
                        }
                    }
                }
                
                let real_expected = real_width * real_height * 4;
                if frame.len() < real_expected { continue; }

                // 1. Calculate Hash to detect changes
                let mut hasher = Hasher::new();
                hasher.update(&frame[..real_expected]);
                let current_hash = hasher.finalize();

                if current_hash == last_frame_hash {
                    std::thread::sleep(Duration::from_millis(10)); 
                    continue;
                }
                last_frame_hash = current_hash;

                let mut buffer = Vec::new();
                let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 40);
                
                let mut rgb_data = vec![0u8; real_width * real_height * 3];
                for (i, chunk) in frame[..real_expected].chunks_exact(4).enumerate() {
                    rgb_data[i * 3]     = chunk[2];
                    rgb_data[i * 3 + 1] = chunk[1];
                    rgb_data[i * 3 + 2] = chunk[0];
                }

                if let Some(img) = RgbImage::from_raw(real_width as u32, real_height as u32, rgb_data) {
                    if let Ok(_) = encoder.encode_image(&img) {
                        if frame_sent % 30 == 0 {
                            tracing::info!("Encoded JPEG! Buffer len: {}, width: {}, height: {}", buffer.len(), width, height);
                        }
                        let mut payload = vec![0x01]; // Video Type
                        payload.extend_from_slice(&buffer);
                        // Using try_send means we don't block the video thread 
                        // if the network loop is lagging behind. Piling up frames 
                        // creates latency anyway. Dropping a frame is better.
                        if let Err(_) = frame_tx.try_send(payload) {
                            // Channel is full or disconnected, drop frame
                        } else {
                            frame_sent += 1;
                        }
                    } else {
                        tracing::warn!("Failed to encode image!");
                    }
                } else {
                    tracing::warn!("Failed to create RgbImage from raw! expected raw len: {}, got: {}", width * height * 3, width * height * 3);
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
