use gloo::console::{debug, info};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

mod services;
pub(crate) use self::services::{
    CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardMapping, KeyboardService,
    TobogganApi,
};

mod app;
use crate::app::App;

mod components;
pub(crate) use crate::components::{
    ToastType, TobogganFooterElement, TobogganHelpElement, TobogganPresenterElement,
    TobogganQuakeTerminalElement, TobogganSlideElement, TobogganToastElement, WasmElement,
};

mod config;
pub use crate::config::*;

#[macro_use]
mod utils;
pub use crate::utils::*;

/// Mounts the deck — what the room looks at.
#[wasm_bindgen]
pub fn start_app(config: AppConfig, elt: &HtmlElement) {
    console_error_panic_hook::set_once();
    info!("🚀 Staring toboggan-wasm application");
    debug!("🎛️ Configuration\n", format!("{config:#?}"));

    let mut app = App::new(config);
    app.render(elt);
}

/// Mounts the presenter view — what the speaker looks at.
///
/// The same application: same socket, same keyboard, same state handling. Only
/// what surrounds the current slide differs, which is why this is one export
/// away rather than a second client.
#[wasm_bindgen]
pub fn start_presenter_app(config: AppConfig, elt: &HtmlElement) {
    console_error_panic_hook::set_once();
    info!("🎙️ Starting the Toboggan presenter view");
    debug!("🎛️ Configuration\n", format!("{config:#?}"));

    let mut app = App::new(config).into_presenter_view();
    app.render(elt);
}
