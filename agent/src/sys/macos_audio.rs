use anyhow::Result;
use std::sync::Mutex;
use tokio::sync::mpsc::Sender;
use tracing::info;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};

lazy_static! {
    static ref AUDIO_TX: Mutex<Option<Sender<Vec<u8>>>> = Mutex::new(None);
    static ref DROPPED_AUDIO_FRAMES: AtomicUsize = AtomicUsize::new(0);
}

// C-Callback invoked by our Objective-C ScreenCaptureKit helper.
// By default, SCK delivers 32-bit Float PCM (Float32).
extern "C" fn audio_callback(data: *const u8, length: usize) {
    if data.is_null() || length == 0 {
        return;
    }
    
    // length is in bytes. For f32, length must be a multiple of 4.
    let float_count = length / 4;
    let floats = unsafe { std::slice::from_raw_parts(data as *const f32, float_count) };
    
    if let Ok(mut tx_guard) = AUDIO_TX.try_lock() {
        if let Some(tx) = tx_guard.as_mut() {
            // Package for Web UI: [0x02, PCM 16-bit...]
            let mut pcm = Vec::with_capacity(float_count * 2 + 1);
            pcm.push(0x02); // Audio Type
            
            for &f in floats {
                // Convert f32 to i16 (assuming it's strictly between -1.0 and 1.0)
                let s = (f.clamp(-1.0, 1.0) * 32767.0) as i16;
                pcm.extend_from_slice(&s.to_le_bytes());
            }
            
            // Send the async message using try_send to avoid deadlocking or blocking 
            // the core_audio loop if the network is disconnected or lagging.
            if let Err(_) = tx.try_send(pcm) {
                // Channel full or disconnected, drop the audio frame
                let dropped = DROPPED_AUDIO_FRAMES.fetch_add(1, Ordering::Relaxed);
                if dropped == 50 {
                    tracing::warn!("Audio channel full! Over 50 audio packets dropped recently due to backpressure/network lag.");
                    DROPPED_AUDIO_FRAMES.store(0, Ordering::Relaxed);
                }
            }
        }
    }
}

extern "C" {
    fn start_sck_capture(callback: extern "C" fn(*const u8, usize));
}

pub fn start_macos_system_audio_capture(tx: Sender<Vec<u8>>) -> Result<()> {
    info!("Starting macOS ScreenCaptureKit audio capture...");
    
    if let Ok(mut guard) = AUDIO_TX.lock() {
        *guard = Some(tx);
    }
    
    // Start native objective-c capture bridge
    // This function will now block the thread and run the NSRunLoop to process SCK events
    unsafe {
        start_sck_capture(audio_callback);
    }
    
    Ok(())
}
