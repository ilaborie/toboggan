use std::time::Duration;

use toboggan_client::{TobogganConfig, TobogganWebsocketConfig};
use toboggan_core::ClientId;

mod app;
pub use app::App;

mod constants;
mod message;
mod state;
mod styles;
mod views;
mod widgets;

#[must_use]
pub fn build_config(websocket_url: Option<String>, api_url: Option<String>) -> TobogganConfig {
    TobogganConfig {
        client_id: ClientId::new(),
        api_url: api_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
        websocket: TobogganWebsocketConfig {
            websocket_url: websocket_url
                .unwrap_or_else(|| "ws://localhost:8080/api/ws".to_string()),
            max_retries: 5,
            retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(30),
        },
    }
}
