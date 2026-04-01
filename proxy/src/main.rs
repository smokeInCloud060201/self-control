use crate::error::Result;
use axum::{routing::get, Router};
use clap::Parser;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

mod cli;
mod error;
mod handlers;
mod models;

use cli::Args;
use handlers::{health::health_check, ws::ws_handler};
use models::session::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let args = Args::parse();
    let state = Arc::new(AppState {
        sessions: Mutex::new(HashMap::new()),
    });

    // GC task to clean up old sessions
    let gc_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut sessions = gc_state.sessions.lock().unwrap();
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

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/agent/:session_id/:password", get(ws_handler))
        .route("/client/:session_id/:password", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    info!("========================================");
    info!("   SECURE PAIRING PROXY v2.6 (Axum)");
    info!("   Listening on: 0.0.0.0:{}", args.port);
    info!("========================================");

    axum::serve(listener, app).await?;

    Ok(())
}
