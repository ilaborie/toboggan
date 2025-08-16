//! Configuration for the TUI application using toboggan-client.
//!
//! Re-exports the shared configuration types from toboggan-client.

pub use toboggan_client::TobogganConfig as Config;

/// Create config from command line arguments, using toboggan-client defaults
#[must_use]
pub fn build_config(websocket_url: Option<String>, api_url: Option<String>) -> Config {
    let mut config = Config::default();

    if let Some(ws_url) = websocket_url {
        config.websocket.websocket_url = ws_url;
    }

    if let Some(api_url) = api_url {
        config.api_url = api_url;
    }

    config
}
