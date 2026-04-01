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
            let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(50);
            let (response_tx, response_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(10);

            // Audio Capture Task
            let audio_tx_clone = audio_tx.clone();
            std::thread::spawn(move || {
                if let Err(e) = capture::audio::start_audio_capture(audio_tx_clone) {
                    tracing::warn!(error = %e, "Audio capture failed to start");
                }
            });

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

            // Start WebSocket Network Loop for Video/Control
            let server_video = server.clone();
            let state_video = state.clone();
            tokio::spawn(async move {
                let _ = network::ws::start_connection_loop(
                    server_video,
                    port,
                    state_video,
                    event_tx,
                    video_rx,
                    response_rx,
                    "video"
                ).await;
            });

            // Start WebSocket Network Loop for Audio
            let (dummy_event_tx, _) = tokio::sync::mpsc::channel::<ControlEvent>(1);
            let (_, dummy_response_rx) = tokio::sync::mpsc::channel::<serde_json::Value>(1);
            let _ = network::ws::start_connection_loop(
                server,
                port,
                state,
                dummy_event_tx,
                audio_rx,
                dummy_response_rx,
                "audio"
            ).await;
        });
    });
}
