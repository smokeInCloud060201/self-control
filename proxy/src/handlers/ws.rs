use axum::extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::models::session::{AppState, Session};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path((session_id, password)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let role = if req.uri().path().starts_with("/agent") {
        "agent"
    } else {
        "client"
    };

    ws.on_upgrade(move |socket| handle_ws(socket, role.to_string(), session_id, password, state))
}

async fn handle_ws(
    socket: WebSocket,
    role: String,
    session_id: String,
    password: String,
    state: Arc<AppState>,
) {
    let (tx, mut rx) = mpsc::channel::<Message>(256);
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Register Session
    {
        let mut sessions = state.sessions.lock().unwrap();
        let session = sessions.entry(session_id.clone()).or_insert(Session {
            agent: None,
            client: None,
            password: None,
            last_activity: Instant::now(),
        });

        session.last_activity = Instant::now();

        if role == "agent" {
            info!(session_id = %session_id, "[AUTH] Agent linked");
            session.password = Some(password.clone());
            session.agent = Some(tx.clone());

            if session.client.is_some() {
                debug!(session_id = %session_id, "Signaling agent to start: client already present");
                let _ = session.agent.as_ref().unwrap().try_send(Message::Text("{\"type\": \"start_capture\"}".into()));
            }
        } else if role == "client" {
            match &session.password {
                Some(p) if p == &password => {
                    info!(session_id = %session_id, "[AUTH] Client linked");
                    session.client = Some(tx.clone());
                    if let Some(agent_tx) = &session.agent {
                        debug!(session_id = %session_id, "Signaling agent to start: client joined");
                        let _ = agent_tx.try_send(Message::Text("{\"type\": \"start_capture\"}".into()));
                    }
                }
                _ => {
                    warn!(session_id = %session_id, "Authentication failed for client");
                    return;
                }
            }
        }
    }

    let relay_session_id = session_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                debug!(session_id = %relay_session_id, error = %e, "WebSocket send failed");
                break;
            }
        }
    });

    // Relay Loop
    while let Some(Ok(msg)) = ws_receiver.next().await {
        let partner_tx = {
            let mut sessions = state.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(&session_id) {
                session.last_activity = Instant::now();
                if role == "agent" {
                    session.client.clone()
                } else {
                    session.agent.clone()
                }
            } else {
                None
            }
        };

        if let Some(tx_partner) = partner_tx {
            if matches!(msg, Message::Binary(_)) {
                if let Err(e) = tx_partner.try_send(msg) {
                    debug!(session_id = %session_id, error = %e, "Relay buffer full");
                }
            } else {
                let _ = tx_partner.send(msg).await;
            }
        }
    }

    // Cleanup
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Instant::now();
            if role == "agent" {
                info!(session_id = %session_id, "[EXIT] Agent lost");
                session.agent = None;
            } else {
                info!(session_id = %session_id, "[EXIT] Client lost");
                session.client = None;
                if let Some(agent_tx) = &session.agent {
                    debug!(session_id = %session_id, "Signaling agent to stop");
                    let _ = agent_tx.try_send(Message::Text("{\"type\": \"stop_capture\"}".into()));
                }
            }
        }
    }

    send_task.abort();
}
