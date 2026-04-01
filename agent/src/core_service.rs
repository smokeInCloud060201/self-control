use crate::app_state::AppState;
use crate::capture;
use crate::controller;
use crate::models::ControlEvent;
use crate::network;

pub fn start_background_services(
    state: AppState,
    server: String,
    port: u16,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (event_tx, event_rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
            let (video_tx, video_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(50);
            let (response_tx, response_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(10);

            // Audio Capture Task
            // FIXME: Need to route audio over WebRTC later or drop for now
            /* Wait for WebRTC Audio DataChannel implementation
            std::thread::spawn(move || {
                let (dummy_tx, _) = tokio::sync::mpsc::channel::<Vec<u8>>(1);
                if let Err(e) = capture::audio::start_audio_capture(dummy_tx) {
                    tracing::warn!(error = %e, "Audio capture failed to start");
                }
            });
            */

            // Video Capture Task
            let is_streaming_cap = state.is_streaming.clone();
            let display_index_cap = state.display_index.clone();
            let video_tx_clone = video_tx.clone();
            std::thread::spawn(move || {
                capture::video::start_video_capture(is_streaming_cap, display_index_cap, video_tx_clone);
            });

            // Input Control Handler Task
            let controller_state = state.clone();
            tokio::spawn(async move {
                controller::start_handler(event_rx, controller_state, response_tx).await;
            });

            // Start WebSocket Signalling and WebRTC Orchestration Loop
            let _ = network::ws::start_connection_loop(
                server,
                port,
                state.clone(),
                event_tx,
                video_rx,
                response_rx,
            ).await;
        });
    });
}
