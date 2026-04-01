use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub machine_id: String,
    pub password_shared: Arc<Mutex<String>>,
    pub is_streaming: Arc<Mutex<bool>>,
    pub display_index: Arc<Mutex<usize>>,
    pub status: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new(machine_id: String, initial_password: String) -> Self {
        Self {
            machine_id,
            password_shared: Arc::new(Mutex::new(initial_password)),
            is_streaming: Arc::new(Mutex::new(false)),
            display_index: Arc::new(Mutex::new(0)),
            status: Arc::new(Mutex::new("Disconnected".to_string())),
        }
    }
}
