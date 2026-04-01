use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::mpsc;

pub type MsgSender = mpsc::Sender<Message>;

pub struct Session {
    pub agent: Option<mpsc::Sender<Message>>,
    pub client: Option<mpsc::Sender<Message>>,
    pub password: Option<String>,
    pub last_activity: Instant,
}

pub struct AppState {
    pub sessions: Mutex<HashMap<String, Session>>,
}
