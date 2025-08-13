use std::{cell::RefCell, mem, rc::Rc};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use gloo::console::error;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;

use toboggan_core::{Command, Renderer, State};

use crate::{
    AppConfig, CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardService,
    ToastType, TobogganApi, TobogganFooterElement, TobogganNavigationElement, TobogganSlideElement,
    TobogganToastElement, WasmElement, create_html_element, play_tada,
};

#[derive(Default)]
struct TobogganAppElement {
    navigation: TobogganNavigationElement,
    slide: TobogganSlideElement,
    footer: TobogganFooterElement,
    toast: TobogganToastElement,
}

pub struct App {
    api: Rc<TobogganApi>,
    kbd: KeyboardService,
    com: Rc<RefCell<CommunicationService>>,
    client_id: toboggan_core::ClientId,

    elt: Rc<RefCell<TobogganAppElement>>,

    rx_msg: Option<UnboundedReceiver<CommunicationMessage>>,
    tx_cmd: Option<UnboundedSender<Command>>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let AppConfig {
            client_id,
            api_base_url,
            websocket,
            keymap,
        } = config;
        let api = TobogganApi::new(&api_base_url);
        let api = Rc::new(api);

        let (tx_cmd, rx_cmd) = unbounded();
        let (tx_msg, rx_msg) = unbounded();
        let rx_msg = Some(rx_msg);

        let kbd = KeyboardService::new(tx_cmd.clone(), keymap.unwrap_or_default());
        let com = CommunicationService::new(client_id, websocket, tx_msg, tx_cmd.clone(), rx_cmd);
        let com = Rc::new(RefCell::new(com));

        let elt = TobogganAppElement::default();
        let elt = Rc::new(RefCell::new(elt));

        Self {
            api,
            kbd,
            com,
            client_id,
            elt,
            rx_msg,
            tx_cmd: Some(tx_cmd),
        }
    }
}

impl WasmElement for App {
    fn render(&mut self, host: &HtmlElement) {
        let Some(rx_msg) = self.rx_msg.take() else {
            error!("Illegal state, render should be called once");
            return;
        };

        // Create inner elements
        let mut inner_elt = self.elt.borrow_mut();

        // Set up navigation with command sender
        if let Some(tx_cmd) = &self.tx_cmd {
            inner_elt.navigation.set_command_sender(tx_cmd.clone());
        }

        let elt = create_html_element("header");
        elt.set_class_name("toboggan-navigation");
        inner_elt.navigation.render(&elt);
        host.append_child(&elt).unwrap_throw();

        let elt = create_html_element("section");
        elt.set_class_name("toboggan-slide");
        inner_elt.slide.render(&elt);
        host.append_child(&elt).unwrap_throw();

        let elt = create_html_element("div");
        elt.set_class_name("toboggan-toast");
        inner_elt.toast.render(&elt);
        host.append_child(&elt).unwrap_throw();

        let elt = create_html_element("footer");
        elt.set_class_name("toboggan-footer");
        inner_elt.footer.render(&elt);
        host.append_child(&elt).unwrap_throw();
        mem::drop(inner_elt);

        // Start services
        self.kbd.start();
        let com = Rc::clone(&self.com);
        spawn_local(async move {
            let mut com = com.borrow_mut();
            com.connect();
        });

        // Handle messages
        let tx_cmd = self.tx_cmd.take().unwrap_throw();
        let api = Rc::clone(&self.api);
        let elt = Rc::clone(&self.elt);
        spawn_local(handle_notificiation(
            api,
            rx_msg,
            elt,
            self.client_id,
            tx_cmd,
        ));
    }
}

// CRITICAL: This function runs in browser's async context.
// NEVER use synchronous blocking operations like std::sync::mpsc::Receiver::recv()
// as they will freeze the entire browser. Always use async alternatives.
async fn handle_notificiation(
    api: Rc<TobogganApi>,
    mut rx: UnboundedReceiver<CommunicationMessage>,
    elt: Rc<RefCell<TobogganAppElement>>,
    client_id: toboggan_core::ClientId,
    tx_cmd: UnboundedSender<Command>,
) {
    use futures::StreamExt;
    while let Some(msg) = rx.next().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                handle_connection_status_change(status, &api, &elt, client_id, &tx_cmd).await;
            }

            CommunicationMessage::StateChange { state } => {
                handle_state_change(state, &api, &elt).await;
            }

            CommunicationMessage::Error { error } => {
                let elt = elt.borrow();
                elt.toast.toast(ToastType::Error, &error);
            }
        }
    }
}

async fn handle_connection_status_change(
    status: ConnectionStatus,
    api: &Rc<TobogganApi>,
    elt: &Rc<RefCell<TobogganAppElement>>,
    client_id: toboggan_core::ClientId,
    tx_cmd: &UnboundedSender<Command>,
) {
    // Update navigation status
    {
        let mut elt_mut = elt.borrow_mut();
        elt_mut
            .navigation
            .set_connection_status(Some(status.clone()));
    }

    // Handle specific status types
    match &status {
        ConnectionStatus::Connecting => {
            let elt_ref = elt.borrow();
            elt_ref
                .toast
                .toast(ToastType::Info, "Connecting to server...");
        }
        ConnectionStatus::Connected => {
            // Show success toast
            {
                let elt_ref = elt.borrow();
                elt_ref
                    .toast
                    .toast(ToastType::Success, "Connected to server");
            }

            // Register with the server
            let register_cmd = Command::Register {
                client: client_id,
                renderer: Renderer::Html,
            };
            let _ = tx_cmd.unbounded_send(register_cmd);

            // Fetch and update talk information
            match api.get_talk().await {
                Ok(talk) => {
                    let mut elt_mut = elt.borrow_mut();
                    elt_mut.footer.set_content(Some(talk.footer.clone()));
                    elt_mut.navigation.set_slide_count(Some(talk.titles.len()));
                    elt_mut.navigation.set_talk(Some(talk));
                }
                Err(err) => {
                    error!("Fail to fetch the talk:", err.to_string());
                }
            }
        }
        ConnectionStatus::Closed => {
            let elt_ref = elt.borrow();
            elt_ref.toast.toast(ToastType::Error, "Connection closed");
        }
        ConnectionStatus::Reconnecting {
            attempt,
            max_attempt,
            delay,
        } => {
            let elt_ref = elt.borrow();
            let message = format!(
                "Reconnecting in {}s ({attempt}/{max_attempt})",
                delay.as_secs()
            );
            elt_ref.toast.toast(ToastType::Warning, &message);
        }
        ConnectionStatus::Error { message } => {
            let elt_ref = elt.borrow();
            elt_ref.toast.toast(ToastType::Error, message);
        }
    }
}

async fn handle_state_change(
    state: State,
    api: &Rc<TobogganApi>,
    elt: &Rc<RefCell<TobogganAppElement>>,
) {
    // Update navigation state
    {
        let mut elt_mut = elt.borrow_mut();
        elt_mut.navigation.set_state(Some(state.clone()));
    }

    // Handle completion state
    if matches!(&state, State::Done { .. }) {
        let elt_ref = elt.borrow();
        elt_ref.toast.toast(ToastType::Success, "🎉 Done");
        play_tada();
    }

    // Update current slide
    if let Some(slide_id) = state.current() {
        match api.get_slide(slide_id).await {
            Ok(slide) => {
                let mut elt_mut = elt.borrow_mut();
                elt_mut.slide.set_slide(Some(slide));
            }
            Err(err) => {
                error!(
                    "Fail to fetch the slide",
                    slide_id.to_string(),
                    "because:",
                    err.to_string()
                );
            }
        }
    } else {
        let mut elt_mut = elt.borrow_mut();
        elt_mut.slide.set_slide(None);
    }
}
