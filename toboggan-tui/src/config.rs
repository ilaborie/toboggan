//! Configuration for the TUI application using toboggan-client.
//!
//! Re-exports the shared configuration types from toboggan-client.

pub use toboggan_client::TobogganConfig as Config;
use toboggan_core::ClientConfig;

/// Create config from command line arguments, using toboggan-client defaults
#[must_use]
pub fn build_config(websocket_url: Option<String>, api_url: Option<String>) -> Config {
    match (websocket_url, api_url) {
        (Some(ws_url), Some(api_url)) => Config::new(api_url, ws_url),
        (Some(ws_url), None) => {
            let config = Config::default();
            Config::new(config.api_url().to_string(), ws_url)
        }
        (None, Some(api_url)) => {
            let config = Config::default();
            Config::new(api_url, config.websocket_url().to_string())
        }
        (None, None) => Config::default(),
    }
}
