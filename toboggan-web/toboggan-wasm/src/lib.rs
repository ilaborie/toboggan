use gloo::console::debug;
use gloo::console::info;

use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

mod services;
pub(crate) use self::services::{
    CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardMapping, KeyboardService,
    TobogganApi,
};

mod app;

mod components;
use crate::app::App;
pub(crate) use crate::components::{
    ToastType, TobogganFooterElement, TobogganNavigationElement, TobogganSlideElement,
    TobogganToastElement, WasmElement,
};

mod config;
pub use crate::config::*;

mod utils;
pub use crate::utils::*;

#[wasm_bindgen]
pub fn start_app(config: AppConfig, elt: &HtmlElement) {
    console_error_panic_hook::set_once();
    info!("🚀 Staring toboggan-wasm application");
    debug!("🎛️ Configuration\n", format!("{config:#?}"));

    let mut app = App::new(config);
    app.render(elt);
}
