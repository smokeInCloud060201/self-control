use anyhow::Result;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_hdr_async_with_config;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use tracing::{info, warn, error, debug, instrument, Level};
use tracing_subscriber::EnvFilter;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Authentication failed for {0}: Incorrect password or session id")]
    AuthFailed(String),
    #[error("Session {0} not found")]
    SessionNotFound(String),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 8080, env = "PROXY_PORT")]
    port: u16,
}

type MsgSender = mpsc::Sender<Message>;

struct Session {
    agent: Option<MsgSender>,
    client: Option<MsgSender>,
    password: Option<String>,
    last_activity: Instant,
}

lazy_static::lazy_static! {
    static ref SESSIONS: Arc<Mutex<HashMap<String, Session>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("========================================");
    info!("   SECURE PAIRING PROXY v2.5");
    info!("   Listening on: {}", addr);
    info!("========================================");

    // GC task to clean up old sessions
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut sessions = SESSIONS.lock().unwrap();
            sessions.retain(|id, s| {
                if s.agent.is_none() && s.client.is_none() && s.last_activity.elapsed().as_secs() > 300 {
                    info!(session_id = %id, "[GC] Purging stale session");
                    false
                } else {
                    true
                }
            });
        }
    });

    while let Ok((stream, _)) = listener.accept().await {
        let _ = stream.set_nodelay(true);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                if !e.to_string().contains("Broken pipe") && !e.to_string().contains("Connection reset") {
                    error!(error = %e, "Connection handler failed");
                }
            }
        });
    }

    Ok(())
}

#[instrument(skip(raw_stream), fields(remote_addr))]
async fn handle_connection(raw_stream: TcpStream) -> Result<()> {
    let mut role = String::new();
    let mut session_id = String::new();
    let mut password = String::new();

    let callback = |req: &Request, response: Response| {
        let path = req.uri().path().trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            role = parts[0].to_string();
            session_id = parts[1].to_string();
            if parts.len() >= 3 {
                password = parts[2].to_string();
            }
        }
        Ok(response)
    };

    let config = WebSocketConfig {
        max_message_size: Some(128 * 1024 * 1024),
        max_frame_size: Some(32 * 1024 * 1024),
        ..Default::default()
    };

    let ws_stream = accept_hdr_async_with_config(raw_stream, callback, Some(config)).await?;
    let (tx, mut rx) = mpsc::channel::<Message>(256); 
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Register Session
    {
        let mut sessions = SESSIONS.lock().unwrap();
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
                    return Err(ProxyError::AuthFailed(session_id).into());
                }
            }
        } else {
            warn!(role = %role, "Invalid role connection attempt");
            return Err(ProxyError::Internal(format!("Invalid role: {}", role)).into());
        }
    }

    let relay_session_id = session_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                debug!(session_id = %relay_session_id, error = %e, "WebSocket send failed, terminating relay task");
                break; 
            }
        }
    });

    // Relay Loop
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Ping(p)) => { 
                let _ = tx.send(Message::Pong(p)).await; 
            }
            Ok(msg) => {
                let partner_tx = {
                    let mut sessions = SESSIONS.lock().unwrap();
                    if let Some(session) = sessions.get_mut(&session_id) {
                        session.last_activity = Instant::now();
                        if role == "agent" { session.client.clone() } else { session.agent.clone() }
                    } else { None }
                };

                if let Some(tx_partner) = partner_tx {
                    if msg.is_binary() {
                        // Binary relay (frames) uses try_send to avoid blocking on slow consumers
                        if let Err(e) = tx_partner.try_send(msg) {
                            debug!(session_id = %session_id, error = %e, "Relay buffer full, dropping binary frame");
                        }
                    } else {
                        // Text relay (control/input) is guaranteed
                        let _ = tx_partner.send(msg).await;
                    }
                }
            }
            Err(e) => {
                debug!(session_id = %session_id, error = %e, "WebSocket receive error, terminating relay loop");
                break;
            }
        }
    }

    // Cleanup
    {
        let mut sessions = SESSIONS.lock().unwrap();
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Instant::now();
            if role == "agent" {
                info!(session_id = %session_id, "[EXIT] Agent lost");
                session.agent = None;
            } else {
                info!(session_id = %session_id, "[EXIT] Client lost");
                session.client = None;
                if let Some(agent_tx) = &session.agent {
                    debug!(session_id = %session_id, "Signaling agent to stop: client left");
                    let _ = agent_tx.try_send(Message::Text("{\"type\": \"stop_capture\"}".into()));
                }
            }
        }
    }

    send_task.abort();
    Ok(())
}
