use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub websocket_url: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            websocket_url: "ws://localhost:3000/api/ws".to_string(),
            max_retries: 5,
            retry_delay_ms: 1000,
        }
    }
}

impl Config {
    pub fn load(config_path: Option<&str>, url_override: Option<String>) -> Result<Self> {
        let mut config = if let Some(path) = config_path {
            Self::load_from_file(path)?
        } else {
            Self::load_default()?
        };

        if let Some(url) = url_override {
            config.websocket_url = url;
        }

        Ok(config)
    }

    fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    fn load_default() -> Result<Self> {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("toboggan").join("desktop.toml");
            if config_path.exists() {
                return Self::load_from_file(config_path.to_string_lossy().as_ref());
            }
        }
        Ok(Self::default())
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
