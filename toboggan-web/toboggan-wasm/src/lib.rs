use gloo::console::{debug, info};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

mod services;
pub(crate) use self::services::mirror::MirrorPane;
pub(crate) use self::services::{
    CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardMapping, KeyboardService,
    TobogganApi,
};

mod app;
use crate::app::App;

mod components;
pub(crate) use crate::components::{
    MirrorApp, ToastType, TobogganFooterElement, TobogganHelpElement, TobogganPresenterElement,
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

/// Mounts a mirror of the deck — `/run`, in an iframe, painted by the page that
/// framed it.
///
/// Takes no [`AppConfig`] because it needs none: a mirror opens no socket and
/// fetches nothing. Everything it draws arrives by `postMessage` from the
/// presenter view, which already holds all of it.
///
/// `pane` is the raw `?mirror=` value. An unknown one mounts nothing rather
/// than guessing, because the two panes differ in what they show.
#[wasm_bindgen]
pub fn start_mirror_app(elt: &HtmlElement, pane: &str) {
    console_error_panic_hook::set_once();

    let Some(pane) = MirrorPane::from_str(pane) else {
        gloo::console::error!("Not a mirror pane:", pane);
        return;
    };
    info!("🪞 Starting a Toboggan deck mirror:", pane.as_str());

    let mut app = MirrorApp::new(pane);
    app.render(elt);
}
