use crate::error::Result;
#[cfg(not(target_os = "macos"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(not(target_os = "macos"))]
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
#[cfg(not(target_os = "macos"))]
use tracing::{debug, error, info, warn};
#[cfg(not(target_os = "macos"))]
use std::time::Duration;

#[cfg(target_os = "macos")]
pub fn start_audio_capture(tx: Sender<Vec<u8>>) -> Result<()> {
    crate::sys::macos_audio::start_macos_system_audio_capture(tx)
}

#[cfg(not(target_os = "macos"))]
pub fn start_audio_capture(tx: Sender<Vec<u8>>) -> Result<()> {
    start_cpal_audio_capture(tx)
}

#[cfg(not(target_os = "macos"))]
fn start_cpal_audio_capture(tx: Sender<Vec<u8>>) -> Result<()> {
    let host = cpal::default_host();
    
    // Diagnostic: Log all available audio devices with their capabilities
    if let Ok(devices) = host.devices() {
        info!("--- Available Audio Devices ---");
        for (i, d) in devices.enumerate() {
            if let Ok(name) = d.name() {
                let has_input = d.supported_input_configs().map(|mut c| c.next().is_some()).unwrap_or(false);
                let has_output = d.supported_output_configs().map(|mut c| c.next().is_some()).unwrap_or(false);
                info!("  {}. {} [Input: {}, Output: {}]", i, name, has_input, has_output);
            }
        }
    }

    // Refined device selection: Prioritize loopback/virtual devices
    let mut is_loopback = false;
    let device: cpal::Device = {
        let devices = host.devices().map_err(|e| anyhow::anyhow!("Failed to list audio devices: {}", e))?;
        let mut best_device: Option<cpal::Device> = None;
        #[cfg(target_os = "windows")]
        let mut fallback_device: Option<cpal::Device> = None;
        #[cfg(not(target_os = "windows"))]
        let fallback_device: Option<cpal::Device> = None;

        for d in devices {
            if let Ok(name) = d.name() {
                let name_lower = name.to_lowercase();
                
                // Prioritize known loopback/virtual devices
                if (name_lower.contains("loopback") || 
                   name_lower.contains("blackhole") || 
                   name_lower.contains("soundflower") ||
                   name_lower.contains("virtual") ||
                   name_lower.contains("monitor") ||
                   name_lower.contains("stereo mix")) && 
                   !name_lower.contains("mic") && 
                   !name_lower.contains("microphone") {
                    best_device = Some(d);
                    is_loopback = true;
                    break;
                }
                
                // Track default output as a strong fallback for Windows Loopback
                #[cfg(target_os = "windows")]
                {
                    if let Some(default_out) = host.default_output_device() {
                        if let Ok(out_name) = default_out.name() {
                            if name == out_name {
                                fallback_device = Some(d.clone());
                            }
                        }
                    }
                }
            }
        }

        if let Some(d) = best_device {
            info!("Found system audio loopback device: {}", d.name().unwrap_or_default());
            d
        } else if let Some(d) = fallback_device {
            info!("No dedicated loopback found, using default output for WASAPI Loopback: {}", d.name().unwrap_or_default());
            is_loopback = true;
            d
        } else {
            let d = host.default_input_device().ok_or_else(|| anyhow::anyhow!("No default audio device found"))?;
            let name = d.name().unwrap_or_else(|_| "Unknown".to_string());
            #[cfg(target_os = "macos")]
            {
                warn!("ADVICE: macOS cannot capture system audio without a virtual driver. Please install 'BlackHole 2ch' (brew install blackhole-2ch) and set it as your system output.");
            }
            if name.to_lowercase().contains("mic") || name.to_lowercase().contains("input") || name.to_lowercase().contains("external") {
                warn!("Capture is using a POTENTIAL MICROPHONE: '{}'. Application sounds (Spotify/YouTube) will likely be mixed with background room noise.", name);
            } else {
                info!("Using default audio device: {}", name);
            }
            d
        }
    };

    info!("Using {} (Loopback: {})", device.name().unwrap_or_default(), is_loopback);

    // Try to find a working configuration
    let mut supported_configs: Vec<cpal::SupportedStreamConfigRange> = Vec::new();
    if let Ok(configs) = device.supported_input_configs() {
        for config in configs {
            supported_configs.push(config);
        }
    }

    if supported_configs.is_empty() {
        // Fallback: Try output configs for loopback support
        if let Ok(output_configs) = device.supported_output_configs() {
            for config in output_configs {
                supported_configs.push(config);
            }
        }
    }

    if supported_configs.is_empty() {
        anyhow::bail!("No supported audio configs found for device");
    }

    let mut stream: Option<cpal::Stream> = None;
    let tx = Arc::new(tx);

    for supported_config in supported_configs {
        let config = supported_config.with_max_sample_rate();
        let sample_format = config.sample_format();
        let channels = config.channels();
        let stream_config: cpal::StreamConfig = config.into();

        debug!(format = ?sample_format, channels = channels, rate = stream_config.sample_rate.0, "Attempting audio config");

        let tx_clone = tx.clone();
        let stream_res: std::result::Result<cpal::Stream, cpal::BuildStreamError> = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    let mut pcm = Vec::with_capacity(data.len() / (channels as usize) * 2 + 1);
                    pcm.push(0x02); // Audio Type
                    // Convert to Mono i16
                    for chunk in data.chunks_exact(channels as usize) {
                        let avg: f32 = chunk.iter().sum::<f32>() / (channels as f32);
                        let s = (avg.clamp(-1.0, 1.0) * 32767.0) as i16;
                        pcm.extend_from_slice(&s.to_le_bytes());
                    }
                    let _ = tx_clone.blocking_send(pcm);
                },
                |err| error!("Audio stream error: {}", err),
                None
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let mut pcm = Vec::with_capacity(data.len() / (channels as usize) * 2 + 1);
                    pcm.push(0x02); // Audio Type
                    for chunk in data.chunks_exact(channels as usize) {
                        let avg: i32 = chunk.iter().map(|&x| x as i32).sum::<i32>() / (channels as i32);
                        let s = avg as i16;
                        pcm.extend_from_slice(&s.to_le_bytes());
                    }
                    let _ = tx_clone.blocking_send(pcm);
                },
                |err| error!("Audio stream error: {}", err),
                None
            ),
            _ => {
                debug!("Skipping unsupported sample format: {:?}", sample_format);
                continue;
            }
        };

        match stream_res {
            Ok(s) => {
                if let Ok(_) = s.play() {
                    info!(rate = stream_config.sample_rate.0, channels = channels, "Audio capture started successfully (System Loopback)");
                    stream = Some(s);
                    break;
                }
            }
            Err(e) => {
                debug!(error = %e, "Failed to build audio stream with this config, trying next...");
            }
        }
    }

    let _stream = stream.ok_or_else(|| anyhow::anyhow!("Failed to start audio capture with any supported config"))?;
    
    // Keep the stream alive
    loop {
        std::thread::sleep(Duration::from_secs(10));
    }
}
