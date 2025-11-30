use std::cell::RefCell;
use std::rc::Rc;

use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gloo::console::{debug, error, info};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;

use toboggan_core::{Command, State};

use crate::{
    AppConfig, CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardService,
    StateClassMapper, ToastType, TobogganApi, TobogganFooterElement, TobogganSlideElement,
    TobogganToastElement, WasmElement, create_html_element, inject_head_html, play_tada,
};

/// Holds metadata about the presentation
#[derive(Debug, Clone, Default)]
struct PresentationMeta {
    /// Total number of slides in the presentation
    total_slides: usize,
}

/// Tracks state recovery information for reconnection scenarios
#[derive(Debug, Clone, Default)]
struct RecoveryState {
    /// Last known state before disconnection
    last_known_state: Option<State>,
    /// Whether we're waiting to attempt state restoration after reconnection
    pending_restoration: bool,
}

#[derive(Default)]
struct TobogganElements {
    slide: TobogganSlideElement,
    footer: TobogganFooterElement,
    toast: TobogganToastElement,
}

pub struct App {
    api: Rc<TobogganApi>,
    kbd: KeyboardService,
    com: Rc<RefCell<CommunicationService>>,
    client_id: toboggan_core::ClientId,
    elements: Rc<RefCell<TobogganElements>>,
    rx_msg: Option<UnboundedReceiver<CommunicationMessage>>,
    rx_action: Option<UnboundedReceiver<Command>>,
    tx_cmd: Option<UnboundedSender<Command>>,
    root_element: Option<Rc<HtmlElement>>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let AppConfig {
            client_id,
            api_base_url,
            websocket,
            keymap,
        } = config;

        let api = Rc::new(TobogganApi::new(&api_base_url));
        let (tx_cmd, rx_cmd) = unbounded();
        let (tx_msg, rx_msg) = unbounded();
        let (tx_action, rx_action) = unbounded();

        let kbd = KeyboardService::new(tx_action, keymap.unwrap_or_default());
        let com = CommunicationService::new(client_id, websocket, tx_msg, tx_cmd.clone(), rx_cmd);
        let com = Rc::new(RefCell::new(com));

        Self {
            api,
            kbd,
            com,
            client_id,
            elements: Rc::new(RefCell::new(TobogganElements::default())),
            rx_msg: Some(rx_msg),
            rx_action: Some(rx_action),
            tx_cmd: Some(tx_cmd),
            root_element: None,
        }
    }
}

impl WasmElement for App {
    fn render(&mut self, host: &HtmlElement) {
        let Some(rx_msg) = self.rx_msg.take() else {
            error!("Render should be called only once");
            return;
        };
        let Some(rx_action) = self.rx_action.take() else {
            error!("Render should be called only once");
            return;
        };

        // Store root element for state class updates
        let root_element = Rc::new(host.clone());
        self.root_element = Some(root_element.clone());

        // Set initial state class
        let current_classes = host.class_name();
        let new_classes = if current_classes.is_empty() {
            "init".to_string()
        } else {
            format!("{current_classes} init")
        };
        host.set_class_name(&new_classes);

        {
            let mut elements = self.elements.borrow_mut();

            let el = create_html_element("div");
            el.set_class_name("toboggan-slide");
            elements.slide.render(&el);
            host.append_child(&el).unwrap_throw();

            let el = create_html_element("div");
            el.set_class_name("toboggan-toast");
            elements.toast.render(&el);
            host.append_child(&el).unwrap_throw();

            let el = create_html_element("footer");
            el.set_class_name("toboggan-footer");
            elements.footer.render(&el);
            host.append_child(&el).unwrap_throw();
        }

        self.kbd.start();

        let com = Rc::clone(&self.com);
        spawn_local(async move {
            com.borrow_mut().connect();
        });

        let tx_cmd = self.tx_cmd.take().unwrap_throw();
        let presentation_meta = Rc::new(RefCell::new(PresentationMeta::default()));
        spawn_local(handle_messages(
            self.api.clone(),
            rx_msg,
            self.elements.clone(),
            self.client_id,
            tx_cmd.clone(),
            root_element,
            presentation_meta,
        ));

        spawn_local(handle_actions(rx_action, self.elements.clone(), tx_cmd));
    }
}

async fn handle_messages(
    api: Rc<TobogganApi>,
    mut rx: UnboundedReceiver<CommunicationMessage>,
    elements: Rc<RefCell<TobogganElements>>,
    client_id: toboggan_core::ClientId,
    tx_cmd: UnboundedSender<Command>,
    root_element: Rc<HtmlElement>,
    presentation_meta: Rc<RefCell<PresentationMeta>>,
) {
    let recovery_state = Rc::new(RefCell::new(RecoveryState::default()));

    while let Some(msg) = rx.next().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                handle_connection_status(
                    &status,
                    &api,
                    &elements,
                    client_id,
                    &tx_cmd,
                    &recovery_state,
                    &presentation_meta,
                )
                .await;
            }
            CommunicationMessage::StateChange { state } => {
                handle_state_change(
                    state,
                    &api,
                    &elements,
                    &root_element,
                    &tx_cmd,
                    &recovery_state,
                    &presentation_meta,
                )
                .await;
            }
            CommunicationMessage::TalkChange { state } => {
                handle_talk_change(
                    state,
                    &api,
                    &elements,
                    &root_element,
                    &tx_cmd,
                    &recovery_state,
                    &presentation_meta,
                )
                .await;
            }
            CommunicationMessage::Error { error } => {
                elements.borrow().toast.toast(ToastType::Error, &error);
            }
        }
    }
}

async fn handle_connection_status(
    status: &ConnectionStatus,
    api: &Rc<TobogganApi>,
    elements: &Rc<RefCell<TobogganElements>>,
    client_id: toboggan_core::ClientId,
    tx_cmd: &UnboundedSender<Command>,
    recovery_state: &Rc<RefCell<RecoveryState>>,
    presentation_meta: &Rc<RefCell<PresentationMeta>>,
) {
    {
        let elems = elements.borrow();

        match status {
            ConnectionStatus::Connecting => {
                elems
                    .toast
                    .toast(ToastType::Info, "Connecting to server...");
            }
            ConnectionStatus::Connected => {
                elems.toast.toast(ToastType::Success, "Connected to server");
            }
            ConnectionStatus::Closed => {
                elems.toast.toast(ToastType::Error, "Connection closed");
            }
            ConnectionStatus::Reconnecting {
                attempt,
                max_attempt,
                delay,
            } => {
                let message = format!(
                    "Reconnecting in {}s ({attempt}/{max_attempt})",
                    delay.as_secs()
                );
                elems.toast.toast(ToastType::Warning, &message);
            }
            ConnectionStatus::Error { message } => {
                elems.toast.toast(ToastType::Error, message);
            }
        }
    }

    if matches!(status, ConnectionStatus::Connected) {
        // Mark that we should attempt recovery when we receive the next state
        recovery_state.borrow_mut().pending_restoration = true;

        let _ = tx_cmd.unbounded_send(Command::Register { client: client_id });

        if let Ok(talk) = api.get_talk().await {
            // Update presentation metadata with total slides count
            presentation_meta.borrow_mut().total_slides = talk.titles.len();

            let mut elem = elements.borrow_mut();
            elem.footer.set_content(talk.footer.clone());
            drop(elem);

            // Inject custom head HTML if provided
            inject_head_html(talk.head.as_deref());
        } else {
            error!("Failed to fetch talk");
        }
    }
}

async fn handle_state_change(
    state: State,
    api: &Rc<TobogganApi>,
    elements: &Rc<RefCell<TobogganElements>>,
    root_element: &Rc<HtmlElement>,
    tx_cmd: &UnboundedSender<Command>,
    recovery_state: &Rc<RefCell<RecoveryState>>,
    presentation_meta: &Rc<RefCell<PresentationMeta>>,
) {
    // Auto-start presentation when in Init state
    if matches!(state, State::Init) {
        info!("Auto-starting presentation from Init state");
        let _ = tx_cmd.unbounded_send(Command::First);
        return;
    }

    // Try to restore previous slide position after reconnection
    if try_restore_slide_position(&state, elements, tx_cmd, recovery_state) {
        return; // We'll receive a new StateChange after GoTo command
    }

    // Save current state for future reconnection recovery
    recovery_state.borrow_mut().last_known_state = Some(state.clone());

    // Update UI to reflect current state
    update_root_state_class(&state, root_element, presentation_meta);
    update_slide_display(&state, api, elements).await;
    show_completion_toast_if_done(&state, elements);
}

async fn handle_talk_change(
    state: State,
    api: &Rc<TobogganApi>,
    elements: &Rc<RefCell<TobogganElements>>,
    root_element: &Rc<HtmlElement>,
    _tx_cmd: &UnboundedSender<Command>,
    recovery_state: &Rc<RefCell<RecoveryState>>,
    presentation_meta: &Rc<RefCell<PresentationMeta>>,
) {
    info!("📝 Presentation updated, reloading talk metadata");

    // Notify user that presentation was updated
    elements
        .borrow()
        .toast
        .toast(ToastType::Info, "📝 Presentation updated");

    // Re-fetch talk metadata
    match api.get_talk().await {
        Ok(talk) => {
            // Update presentation metadata with total slides count
            presentation_meta.borrow_mut().total_slides = talk.titles.len();

            let mut elem = elements.borrow_mut();
            elem.footer.set_content(talk.footer.clone());
            drop(elem);

            // Inject custom head HTML if provided
            inject_head_html(talk.head.as_deref());
        }
        Err(err) => {
            error!("Failed to refetch talk after TalkChange:", err.to_string());
            elements
                .borrow()
                .toast
                .toast(ToastType::Error, "Failed to reload presentation metadata");
        }
    }

    // Save current state for future re-connection recovery
    recovery_state.borrow_mut().last_known_state = Some(state.clone());

    // Update UI to reflect current state (server has already adjusted slide position)
    update_root_state_class(&state, root_element, presentation_meta);
    update_slide_display(&state, api, elements).await;
    show_completion_toast_if_done(&state, elements);
}

/// Attempts to restore slide position after re-connection
/// Returns true if restoration was attempted (caller should return early)
fn try_restore_slide_position(
    state: &State,
    elements: &Rc<RefCell<TobogganElements>>,
    tx_cmd: &UnboundedSender<Command>,
    recovery_state: &Rc<RefCell<RecoveryState>>,
) -> bool {
    let mut recovery = recovery_state.borrow_mut();

    // Not pending restoration? Nothing to do
    if !recovery.pending_restoration {
        return false;
    }

    recovery.pending_restoration = false;

    // Server has active state? Respect it (server wasn't restarted)
    if !matches!(state, State::Init) {
        debug!(
            "Skipping restoration - server has active state:",
            state.to_css_class()
        );
        return false;
    }

    // Extract last known state or return
    let Some(last_state) = &recovery.last_known_state else {
        return false;
    };

    // Extract slide position from last state or return
    let Some(slide_id) = last_state.current() else {
        return false;
    };

    info!(
        "Attempting to restore to slide",
        slide_id, "after reconnection"
    );

    // Send GoTo command to restore position
    elements.borrow().toast.toast(
        ToastType::Info,
        &format!("Restoring to slide {slide_id}..."),
    );

    if tx_cmd
        .unbounded_send(Command::GoTo { slide: slide_id })
        .is_err()
    {
        error!("Failed to send GoTo command for restoration");
        return false;
    }

    info!("Sent GoTo command to restore to slide", slide_id);
    true
}

/// Updates root element CSS class to reflect current state
fn update_root_state_class(
    state: &State,
    root_element: &HtmlElement,
    presentation_meta: &Rc<RefCell<PresentationMeta>>,
) {
    let state_class = state.to_css_class();
    let current_classes = root_element.class_name();

    // Remove old state classes and add new one
    let classes: Vec<&str> = current_classes
        .split_whitespace()
        .filter(|class| !matches!(*class, "init" | "paused" | "running" | "done"))
        .collect();

    let new_classes = if classes.is_empty() {
        state_class.to_string()
    } else {
        format!("{} {state_class}", classes.join(" "))
    };

    root_element.set_class_name(&new_classes);

    // Update CSS custom properties for slide tracking
    let current_slide = state.current().map_or(0, |idx| idx + 1); // 1-based for display
    let total_slides = presentation_meta.borrow().total_slides;

    let style = root_element.style();
    let _ = style.set_property("--current-slide", &current_slide.to_string());
    let _ = style.set_property("--total-slides", &total_slides.to_string());
}

/// Fetches and displays the slide corresponding to current state
async fn update_slide_display(
    state: &State,
    api: &Rc<TobogganApi>,
    elements: &Rc<RefCell<TobogganElements>>,
) {
    let Some(slide_id) = state.current() else {
        debug!("No current slide, clearing slide component");
        elements.borrow_mut().slide.set_slide(None, 0);
        return;
    };

    let state_class = state.to_css_class();
    let current_step = state.current_step();
    debug!(
        "Fetching slide",
        slide_id, "for state", state_class, "step", current_step
    );

    let Ok(slide) = api.get_slide(slide_id).await else {
        error!("Failed to fetch slide", slide_id);
        return;
    };

    elements
        .borrow_mut()
        .slide
        .set_slide(Some(slide), current_step);
}

/// Shows completion toast if presentation is done
fn show_completion_toast_if_done(state: &State, elements: &Rc<RefCell<TobogganElements>>) {
    if matches!(state, State::Done { .. }) {
        debug!("Showing done toast");
        let elements = elements.borrow();
        elements.toast.toast(ToastType::Success, "🎉 Done");
        play_tada();
    }
}

async fn handle_actions(
    mut rx: UnboundedReceiver<Command>,
    _elements: Rc<RefCell<TobogganElements>>,
    tx_cmd: UnboundedSender<Command>,
) {
    while let Some(cmd) = rx.next().await {
        if tx_cmd.unbounded_send(cmd).is_err() {
            error!("Failed to send command");
        }
    }
}
