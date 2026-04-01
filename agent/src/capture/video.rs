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
    let mut frames_dropped = 0;
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
                let mut stride_bytes = width * 4;

                let expected = width * height * 4;
                if frame.len() < expected { continue; }

                // Retina and display scaling check
                // macOS can return frame buffers with vertical padding. 
                // We use integer rounding of the aspect square root to find the true multiplier (1x, 2x).
                let scale_ratio = frame.len() as f64 / expected as f64;
                let scale = scale_ratio.sqrt().round() as usize;

                if scale >= 1 {
                    real_width = width * scale;
                    real_height = height * scale;
                }

                // Base stride is tightly packed pixels
                stride_bytes = real_width * 4;
                
                // macOS IOSurface typically aligns row bytes to multiples of 64 or 256.
                // If it's not aligned to 64, we pad it.
                if stride_bytes % 64 != 0 {
                    stride_bytes = (stride_bytes + 31) & !31; // 32-byte align fallback
                    if stride_bytes % 64 != 0 {
                        stride_bytes = (stride_bytes + 63) & !63;
                    }
                }
                
                let frame_buffer_size = stride_bytes * real_height;
                if frame.len() < frame_buffer_size { continue; }

                // 1. Calculate Hash to detect changes (including any padding for simplicity)
                let mut hasher = Hasher::new();
                hasher.update(&frame[..frame_buffer_size]);
                let current_hash = hasher.finalize();

                if current_hash == last_frame_hash {
                    std::thread::sleep(Duration::from_millis(10)); 
                    continue;
                }
                last_frame_hash = current_hash;

                let mut buffer = Vec::new();
                let mut encoder = JpegEncoder::new_with_quality(&mut buffer, 25);
                
                let mut rgb_data = vec![0u8; real_width * real_height * 3];
                for row in 0..real_height {
                    let src_offset = row * stride_bytes;
                    let dst_offset = row * real_width * 3;
                    if src_offset + real_width * 4 <= frame.len() {
                        let row_data = &frame[src_offset..src_offset + real_width * 4];
                        for (i, chunk) in row_data.chunks_exact(4).enumerate() {
                            rgb_data[dst_offset + i * 3]     = chunk[2];
                            rgb_data[dst_offset + i * 3 + 1] = chunk[1];
                            rgb_data[dst_offset + i * 3 + 2] = chunk[0];
                        }
                    }
                }

                if let Some(img) = RgbImage::from_raw(real_width as u32, real_height as u32, rgb_data) {
                    if let Ok(_) = encoder.encode_image(&img) {
                        if frame_sent % 30 == 0 {
                            tracing::info!("Encoded JPEG! Buffer len: {}, width: {}, height: {}", buffer.len(), width, height);
                        }

                        if let Err(_) = frame_tx.try_send(buffer.clone()) {
                            frames_dropped += 1;
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
                        info!("[STATUS] Uplink: {} fps ({} dropped frames, LOGIN WINDOW)", frame_sent / 5, frames_dropped);
                    } else {
                        info!("[STATUS] Uplink: {} fps ({} dropped frames)", frame_sent / 5, frames_dropped);
                    }
                    frame_sent = 0;
                    frames_dropped = 0;
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
