use crate::error::Result;
use crate::models::ControlEvent;
use crate::app_state::AppState;
use futures_util::{StreamExt, future::BoxFuture};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, debug, error};

use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub async fn setup_webrtc(
    event_tx: mpsc::Sender<ControlEvent>,
) -> Result<(Arc<webrtc::peer_connection::RTCPeerConnection>, mpsc::Receiver<Arc<RTCDataChannel>>)> {
    
    // Build WebRTC API
    let api = APIBuilder::new().build();

    // Setup Coturn config
    let config = RTCConfiguration {
        ice_servers: vec![
            RTCIceServer {
                urls: vec![
                    "stun:stun.l.google.com:19302".to_owned(),
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let pc = Arc::new(api.new_peer_connection(config).await?);

    pc.on_peer_connection_state_change(Box::new(|s: RTCPeerConnectionState| {
        info!("Peer Connection State has changed: {s}");
        Box::pin(async {})
    }));

    let (video_dc_tx, video_dc_rx) = mpsc::channel::<Arc<RTCDataChannel>>(1);
    
    let event_tx_clone = event_tx.clone();

    // Listen for WebRTC Data Channels created by the browser
    pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let label = d.label().to_owned();
        info!("New DataChannel {} {}", d.label(), d.id());

        let video_dc_tx2 = video_dc_tx.clone();
        let event_tx2 = event_tx_clone.clone();

        let d1 = d.clone();
        d.on_open(Box::new(move || {
            info!("Data channel '{}'-'{}' open.", label, d1.id());
            
            if label == "video_stream" {
                let _ = video_dc_tx2.try_send(d1.clone());
            }

            Box::pin(async {})
        }));

        let label_msg = d.label().to_owned();
        let d2 = d.clone();
        d.on_message(Box::new(move |msg| {
            if label_msg == "input_stream" {
                let data = String::from_utf8_lossy(&msg.data).to_string();
                if let Ok(event) = serde_json::from_str::<ControlEvent>(&data) {
                    let ev_clone = event_tx2.clone();
                    tokio::spawn(async move {
                        let _ = ev_clone.send(event).await;
                    });
                }
            }
            Box::pin(async {})
        }));

        Box::pin(async {})
    }));

    // We return the connection, and a receiver that fires when the video channel is ready
    Ok((pc, video_dc_rx))
}
