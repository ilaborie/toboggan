use gloo::utils::document;
use toboggan_core::ClientId;
use wasm_bindgen::prelude::*;

use crate::KeyboardMapping;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub(crate) url: String,
    pub max_retries: usize,
    pub initial_retry_delay: usize,
    pub max_retry_delay: usize,
}

#[wasm_bindgen]
impl WebSocketConfig {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(url: String) -> Self {
        Self {
            url,
            max_retries: 5,
            initial_retry_delay: 1_000,
            max_retry_delay: 30_000,
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub(crate) client_id: ClientId,
    pub(crate) api_base_url: String,
    pub(crate) websocket: WebSocketConfig,
    pub(crate) keymap: Option<KeyboardMapping>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl AppConfig {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        let client_id = ClientId::new();
        let location = document().location().unwrap_throw();
        let api_base_url = location.origin().unwrap_throw();
        let ws_url = format!("ws://{}/api/ws", location.host().unwrap_throw());
        let websocket = WebSocketConfig::new(ws_url);

        Self {
            client_id,
            api_base_url,
            websocket,
            keymap: None,
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_api_base_url(&mut self, url: String) {
        self.api_base_url = url;
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn api_base_url(&self) -> String {
        self.api_base_url.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_websocket(&mut self, websocket: WebSocketConfig) {
        self.websocket = websocket;
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn websocket(&self) -> WebSocketConfig {
        self.websocket.clone()
    }
}
