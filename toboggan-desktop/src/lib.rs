use toboggan_client::TobogganConfig;
use toboggan_core::ClientConfig;

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
    let mut config = TobogganConfig::default();

    if let Some(ws_url) = websocket_url {
        config = TobogganConfig::new(api_url.as_deref().unwrap_or(config.api_url()), ws_url);
    } else if let Some(api_url) = api_url {
        config = TobogganConfig::new(api_url, config.websocket_url().to_string());
    }

    config
}
