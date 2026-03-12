use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub default_passkey: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_passkey: None,
        }
    }
}

pub fn load_config() -> AppConfig {
    confy::load("selfcontrol", "agent").unwrap_or_default()
}

pub fn save_config(config: &AppConfig) {
    let _ = confy::store("selfcontrol", "agent", config);
}
