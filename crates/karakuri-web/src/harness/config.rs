//! Persistent configuration for the In-Process Agent Harness in localStorage.

use serde::{Deserialize, Serialize};

pub const DEFAULT_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_MODEL: &str = "llama3.2";
const STORAGE_KEY: &str = "karakuri_harness_config";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            model: DEFAULT_MODEL.to_string(),
            api_key: None,
        }
    }
}

impl HarnessConfig {
    /// Loads configuration from browser `localStorage`, falling back to default.
    pub fn load() -> Self {
        if let Some(storage) = get_local_storage() {
            if let Ok(Some(raw)) = storage.get_item(STORAGE_KEY) {
                if let Ok(cfg) = serde_json::from_str::<HarnessConfig>(&raw) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    /// Saves configuration to browser `localStorage`.
    pub fn save(&self) {
        if let Some(storage) = get_local_storage() {
            if let Ok(raw) = serde_json::to_string(self) {
                let _ = storage.set_item(STORAGE_KEY, &raw);
            }
        }
    }
}

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
