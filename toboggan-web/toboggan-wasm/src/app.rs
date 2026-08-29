use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gloo::console::{debug, error, info};
use gloo::utils::document;
use toboggan_core::{ClientId, ClientRole, Command, SlideId, State, TalkResponse};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;

use crate::components::deck::{apply_deck_state, mount_deck};
use crate::{
    AppConfig, CommunicationMessage, CommunicationService, ConnectionStatus, KeyboardService,
    StateClassMapper, ToastType, TobogganApi, TobogganFooterElement, TobogganHelpElement,
    TobogganPresenterElement, TobogganQuakeTerminalElement, TobogganSlideElement,
    TobogganToastElement, WasmElement, create_html_element, inject_head_html,
    install_keyboard_release, play_tada, set_document_lang,
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
    /// Whether a `?slide=N` auto-start is awaiting its first state change. The
    /// server rejects an out-of-range index without changing state, so an error
    /// while this is set means the jump failed and we must start from the first
    /// slide instead of sitting in `Init` with a blank screen.
    pending_url_goto: bool,
}

#[derive(Default)]
struct TobogganElements {
    slide: TobogganSlideElement,
    footer: TobogganFooterElement,
    toast: TobogganToastElement,
    quake: TobogganQuakeTerminalElement,
    help: TobogganHelpElement,
    /// The speaker's own view, on `/presenter` only. `None` on the deck the
    /// room is watching, which is what makes the two pages one app: the
    /// connection, the keyboard and the state handling are shared, and only
    /// what surrounds the current slide differs.
    presenter: Option<TobogganPresenterElement>,
}

/// Everything the message handlers share.
///
/// A struct rather than the same seven handles threaded through four functions
/// that each wanted almost all of them: the signatures had come to differ only
/// by which handle a given one happened not to need, which is not a distinction
/// worth carrying in a parameter list.
///
/// `Cell`/`RefCell` rather than `Rc<RefCell<_>>` per field, because the whole
/// context is shared as one `Rc` — the interior mutability is still per field.
struct Session {
    api: Rc<TobogganApi>,
    elements: Rc<RefCell<TobogganElements>>,
    root_element: Rc<HtmlElement>,
    tx_cmd: UnboundedSender<Command>,
    recovery: RefCell<RecoveryState>,
    meta: RefCell<PresentationMeta>,
    /// The id the server assigned, once it has.
    client_id: Cell<Option<ClientId>>,
    /// What the server granted this client. Starts as presenter: the everyday
    /// deck is served from the machine it is presented on, and the handshake
    /// corrects this within a round trip when it is not. Starting as audience
    /// would make the first keypresses after load do nothing on an ordinary
    /// local deck.
    role: Cell<ClientRole>,
}

impl Session {
    /// Whether this client may send commands that move the deck.
    fn presents(&self) -> bool {
        self.role.get().is_presenter()
    }

    fn toast(&self, kind: ToastType, message: &str) {
        self.elements.borrow().toast.toast(kind, message);
    }
}

pub(crate) struct App {
    api: Rc<TobogganApi>,
    kbd: KeyboardService,
    com: Rc<RefCell<CommunicationService>>,
    elements: Rc<RefCell<TobogganElements>>,
    rx_msg: Option<UnboundedReceiver<CommunicationMessage>>,
    rx_action: Option<UnboundedReceiver<Command>>,
    /// The same channel the keyboard writes to, kept so the presenter view's
    /// on-screen buttons can write to it too. Going through `handle_actions` is
    /// the point: that is the one place a refusal can be *shown*, and a second
    /// path would be a second copy of the rule that can disagree with it.
    tx_action: UnboundedSender<Command>,
    tx_cmd: Option<UnboundedSender<Command>>,
    root_element: Option<Rc<HtmlElement>>,
    presenter_view: bool,
}

impl App {
    pub(crate) fn new(config: AppConfig) -> Self {
        let AppConfig {
            api_base_url,
            websocket,
            keymap,
        } = config;

        let api = Rc::new(TobogganApi::new(&api_base_url));
        let (tx_cmd, rx_cmd) = unbounded();
        let (tx_msg, rx_msg) = unbounded();
        let (tx_action, rx_action) = unbounded();

        let keymap = keymap.unwrap_or_default();
        let mut elements = TobogganElements::default();
        elements.help.set_mapping(keymap.clone());
        let kbd = KeyboardService::new(tx_action.clone(), keymap);
        let com =
            CommunicationService::new("Web Client", websocket, tx_msg, tx_cmd.clone(), rx_cmd);
        let com = Rc::new(RefCell::new(com));

        Self {
            api,
            kbd,
            com,
            elements: Rc::new(RefCell::new(elements)),
            rx_msg: Some(rx_msg),
            rx_action: Some(rx_action),
            tx_action,
            tx_cmd: Some(tx_cmd),
            root_element: None,
            presenter_view: false,
        }
    }

    /// Renders the speaker's view — notes, the next slide, a clock — instead of
    /// the deck.
    ///
    /// The client name changes with it, because `/api/clients` and the
    /// connect/disconnect toasts are how a presenter tells the projector, the
    /// phone and their own second window apart.
    pub(crate) fn into_presenter_view(mut self) -> Self {
        self.presenter_view = true;
        self.com.borrow_mut().set_client_name("Presenter View");
        self
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
            "init".to_owned()
        } else {
            format!("{current_classes} init")
        };
        host.set_class_name(&new_classes);

        {
            let mut elements = self.elements.borrow_mut();
            // Reborrowed so `mount_deck` can take two of its fields at once:
            // through the `RefMut` alone they are the same borrow.
            let elements = &mut *elements;
            if self.presenter_view {
                // Nothing here renders a slide: the presenter's two panes are
                // iframes of the deck itself, each painting the real thing in a
                // document of its own. `elements.slide` and `elements.quake`
                // stay unrendered on this page, which is safe — every one of
                // their setters returns early without a container — and
                // `update_slide_display` does not reach for them here anyway.
                let mut presenter = TobogganPresenterElement::default();
                presenter.set_commands(self.tx_action.clone());
                presenter.render(host);
                elements.presenter = Some(presenter);
            } else {
                elements.slide.set_api_base_url(self.api.base_url());
                mount_deck(host, &mut elements.slide, &mut elements.footer);

                // The quake terminal mounts itself directly under <body>; the
                // host element passed here is unused. render() must run before
                // set_api_base_url since the latter writes into the rendered
                // state.
                let placeholder = create_html_element("div");
                elements.quake.render(&placeholder);
                elements.quake.set_api_base_url(self.api.base_url());
            }

            let el = create_html_element("div");
            el.set_class_name("toboggan-toast");
            elements.toast.render(&el);
            if self.presenter_view {
                // The presenter layout attaches a shadow root to `host` and has
                // no <slot>, so a light-DOM child of `host` never renders. The
                // toast went there anyway, which meant the presenter view could
                // not show *any* message — including "Following along — this
                // client cannot drive the deck", the one that explains why the
                // keys stopped working.
                document()
                    .body()
                    .map(|body| body.append_child(&el))
                    .transpose()
                    .unwrap_throw();
            } else {
                host.append_child(&el).unwrap_throw();
            }

            // The help dialog mounts under <body>; the host is unused. Both
            // views get it — the keys are the same in both.
            let placeholder = create_html_element("div");
            elements.help.render(&placeholder);
        }

        self.kbd.start();
        // The deck's keys stand down while a terminal holds the keyboard, so
        // there has to be a way to hand them back: a click off the terminal, or
        // Shift+Escape for a presenter who has no mouse to hand.
        install_keyboard_release();

        let com = Rc::clone(&self.com);
        spawn_local(async move {
            com.borrow_mut().connect();
        });

        let session = Rc::new(Session {
            api: self.api.clone(),
            elements: self.elements.clone(),
            root_element,
            tx_cmd: self.tx_cmd.take().unwrap_throw(),
            recovery: RefCell::new(RecoveryState::default()),
            meta: RefCell::new(PresentationMeta::default()),
            client_id: Cell::new(None),
            role: Cell::new(ClientRole::Presenter),
        });

        // Register beforeunload listener to send Unregister command
        {
            let session = Rc::clone(&session);
            let window = web_sys::window().unwrap_throw();
            let closure = Closure::<dyn FnMut(_)>::new(move |_: web_sys::BeforeUnloadEvent| {
                if let Some(id) = session.client_id.get() {
                    let _ = session
                        .tx_cmd
                        .unbounded_send(Command::Unregister { client: id });
                }
            });
            window
                .add_event_listener_with_callback("beforeunload", closure.as_ref().unchecked_ref())
                .unwrap_throw();
            closure.forget();
        }

        spawn_local(handle_messages(rx_msg, Rc::clone(&session)));
        spawn_local(handle_actions(rx_action, session));
    }
}

async fn handle_messages(mut rx: UnboundedReceiver<CommunicationMessage>, session: Rc<Session>) {
    while let Some(msg) = rx.next().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                handle_connection_status(&status, &session);
            }
            CommunicationMessage::StateChange { state } => {
                handle_state_change(state, &session).await;
            }
            CommunicationMessage::TalkChange { state } => {
                handle_talk_change(state, &session).await;
            }
            CommunicationMessage::Error { error } => {
                session.toast(ToastType::Error, &error);
                // The only error that can reach us while still in `Init` is a
                // rejected `?slide=N` (the overview links to a slide the deck no
                // longer has). Start from the first slide rather than leaving the
                // page blank.
                let jump_failed = {
                    let mut recovery = session.recovery.borrow_mut();
                    std::mem::take(&mut recovery.pending_url_goto)
                };
                if jump_failed && session.presents() {
                    info!("Slide from URL was rejected, starting from the first slide");
                    let _ = session.tx_cmd.unbounded_send(Command::First);
                }
            }
            CommunicationMessage::Registered {
                client_id: id,
                role,
            } => {
                session.client_id.set(Some(id));
                session.role.set(role);
                // The presenter view's buttons say the same thing the toast
                // does, in the place the speaker is looking: a control that does
                // nothing is worse than no control. They start visible, matching
                // `Session.role`'s default and for its reason — on an ordinary
                // local deck the handshake confirms rather than corrects, and
                // starting hidden would flash them away and back.
                if let Some(presenter) = &session.elements.borrow().presenter {
                    presenter.set_can_drive(role.is_presenter());
                }
                if !role.is_presenter() {
                    session.toast(
                        ToastType::Info,
                        "Following along — this client cannot drive the deck",
                    );
                }
            }
            CommunicationMessage::ClientConnected { name, .. } => {
                session.toast(ToastType::Info, &format!("{name} connected"));
            }
            CommunicationMessage::ClientDisconnected { name, .. } => {
                session.toast(ToastType::Info, &format!("{name} disconnected"));
            }
        }
    }
}

fn handle_connection_status(status: &ConnectionStatus, session: &Rc<Session>) {
    {
        let elems = session.elements.borrow();

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
        session.recovery.borrow_mut().pending_restoration = true;

        // Register command is sent automatically by CommunicationService

        // Deliberately *not* awaited here. `handle_messages` reads one message
        // at a time, so awaiting this fetch left the `Registered` and `State`
        // frames — the ones that decide which slide to paint — sitting unread
        // for a whole HTTP round trip, in front of the first slide. Nothing on
        // that path needs the talk metadata: it is the footer, the slide count
        // and the custom head, all of which can land a moment later.
        spawn_local(fetch_talk_metadata(Rc::clone(session)));
    }
}

/// Hands freshly fetched talk metadata to whichever view is mounted.
///
/// The two callers — the first fetch and a live reload — used to carry a copy of
/// this each, and both needed the same correction, which is a good enough
/// argument on its own.
fn apply_talk(talk: &TalkResponse, session: &Session) {
    session.meta.borrow_mut().total_slides = talk.titles.len();

    let mut elem = session.elements.borrow_mut();
    // Reborrowed so the two arms can touch different fields: through the
    // `RefMut` alone, `&elements.presenter` and `&mut elements.footer` are the
    // same borrow.
    let elements = &mut *elem;
    // The deck's `_head.html` and footer belong to the deck, and on this page
    // the mirrors *are* the deck — each renders them in a document of its own.
    // The head used to be injected here as well, which is how the packaged
    // guide's `main { background: … }` came to paint the speaker's chrome with
    // the projector's backdrop: `<main>` is the presenter shell's shadow host,
    // and a rule in the outer document outranks `:host` whatever its
    // specificity.
    let mirrored = if let Some(presenter) = &elements.presenter {
        presenter.set_talk(talk);
        true
    } else {
        elements.footer.set_content(talk.footer.clone());
        false
    };
    drop(elem);

    if !mirrored {
        inject_head_html(talk.head.as_deref());
    }
    // Both views: the presenter's notes are in the deck's language too.
    set_document_lang(talk.lang.as_deref());
}

/// Fills the presenter's slide picker with the deck as plain text.
///
/// The one caller of `/api/outline`, and only on the page that has a picker:
/// the response is every slide's body and notes again, which nothing that shows
/// a single slide has any use for. A failure costs the picker its search and
/// nothing else — it still opens, still shows the deck and still jumps.
async fn fetch_outline(session: &Session) {
    if session.elements.borrow().presenter.is_none() {
        return;
    }
    match session.api.get_outline().await {
        Ok(outline) => {
            if let Some(presenter) = &session.elements.borrow().presenter {
                presenter.set_outline(&outline.slides);
            }
        }
        Err(err) => error!(
            "Failed to fetch the slide outline:",
            err.to_string(),
            "— the slide picker cannot be searched"
        ),
    }
}

/// Fills in the deck's metadata: footer, slide count, presenter plan, custom head.
async fn fetch_talk_metadata(session: Rc<Session>) {
    match session.api.get_talk().await {
        Ok(talk) => {
            apply_talk(&talk, &session);
            fetch_outline(&session).await;
        }
        // Report what actually failed, and what the presenter will see: the
        // slide counter stays at 0 and the deck's `_head.html` (fonts, custom
        // CSS) is never injected, so the deck renders unstyled.
        Err(err) => error!(
            "Failed to fetch talk:",
            err.to_string(),
            "— slide count and custom head styles are unavailable"
        ),
    }
}

async fn handle_state_change(state: State, session: &Session) {
    // Auto-start presentation when in Init state. If the URL carries `?slide=N`
    // (e.g. opened from the slide overview), jump straight to that slide.
    //
    // Starting the deck is itself a command, so an audience client must not try:
    // the server would refuse it and the page would sit in `Init` — blank —
    // for the whole talk. It waits for the presenter to start instead, and the
    // broadcast state change brings it along.
    if matches!(state, State::Init) {
        if !session.presents() {
            info!("Waiting for the presenter to start");
            session.toast(ToastType::Info, "Waiting for the presenter to start…");
            return;
        }
        if let Some(index) = slide_from_url() {
            info!("Starting at slide from URL");
            session.recovery.borrow_mut().pending_url_goto = true;
            let _ = session.tx_cmd.unbounded_send(Command::GoTo {
                slide: SlideId::new(index),
            });
        } else {
            info!("Auto-starting presentation from Init state");
            let _ = session.tx_cmd.unbounded_send(Command::First);
        }
        return;
    }

    // We left `Init`, so the URL jump landed and needs no fallback.
    session.recovery.borrow_mut().pending_url_goto = false;

    // Try to restore previous slide position after reconnection. Also a command,
    // so also presenter-only — an audience client simply takes whatever state
    // the server sends it on reconnect, which is the right answer anyway.
    if session.presents() && try_restore_slide_position(&state, session) {
        return; // We'll receive a new StateChange after GoTo command
    }

    // Save current state for future reconnection recovery
    session.recovery.borrow_mut().last_known_state = Some(state.clone());

    // Update UI to reflect current state
    update_root_state_class(&state, session);
    update_slide_display(&state, session).await;
    show_completion_toast_if_done(&state, session);
}

async fn handle_talk_change(state: State, session: &Session) {
    info!("📝 Presentation updated, reloading talk metadata");

    // Notify user that presentation was updated
    session.toast(ToastType::Info, "📝 Presentation updated");

    // Re-fetch talk metadata
    match session.api.get_talk().await {
        Ok(talk) => {
            apply_talk(&talk, session);
            fetch_outline(session).await;
        }
        Err(err) => {
            error!("Failed to refetch talk after TalkChange:", err.to_string());
            session.toast(ToastType::Error, "Failed to reload presentation metadata");
        }
    }

    // Save current state for future re-connection recovery
    session.recovery.borrow_mut().last_known_state = Some(state.clone());

    // Update UI to reflect current state (server has already adjusted slide position)
    update_root_state_class(&state, session);
    update_slide_display(&state, session).await;
    show_completion_toast_if_done(&state, session);
}

/// Reads a `slide=N` parameter from the page URL query string, if present.
///
/// Used by the slide overview's click-to-run links (`/run?slide=N`).
fn slide_from_url() -> Option<usize> {
    let search = web_sys::window()?.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(&search);
    query.split('&').find_map(|pair| {
        pair.strip_prefix("slide=")
            .and_then(|value| value.parse::<usize>().ok())
    })
}

/// Attempts to restore slide position after re-connection
/// Returns true if restoration was attempted (caller should return early)
fn try_restore_slide_position(state: &State, session: &Session) -> bool {
    let mut recovery = session.recovery.borrow_mut();

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
        slide_id.display_number(),
        "after reconnection"
    );

    // Send GoTo command to restore position
    session.toast(
        ToastType::Info,
        &format!("Restoring to slide {slide_id}..."),
    );

    if session
        .tx_cmd
        .unbounded_send(Command::GoTo { slide: slide_id })
        .is_err()
    {
        error!("Failed to send GoTo command for restoration");
        return false;
    }

    info!(
        "Sent GoTo command to restore to slide",
        slide_id.display_number()
    );
    true
}

/// Updates root element CSS class to reflect current state
fn update_root_state_class(state: &State, session: &Session) {
    apply_deck_state(
        session.root_element.as_ref(),
        state.to_css_class(),
        state.current().map_or(0, SlideId::display_number),
        session.meta.borrow().total_slides,
    );
}

/// Fetches and displays the slide corresponding to current state
async fn update_slide_display(state: &State, session: &Session) {
    let Some(slide_id) = state.current() else {
        debug!("No current slide, clearing slide component");
        let mut elements = session.elements.borrow_mut();
        let elements = &mut *elements;
        if let Some(presenter) = &elements.presenter {
            presenter.set_state(state, None);
        } else {
            elements.slide.set_slide(None, 0);
        }
        return;
    };

    let state_class = state.to_css_class();
    let current_step = state.current_step();
    debug!(
        "Fetching slide",
        slide_id.display_number(),
        "for state",
        state_class,
        "step",
        current_step
    );

    let Ok(slide) = session.api.get_slide(slide_id).await else {
        error!("Failed to fetch slide", slide_id.display_number());
        return;
    };

    {
        let mut elems = session.elements.borrow_mut();
        let elems = &mut *elems;
        // The slide goes by value: on the presenter page the slide component is
        // never rendered, so there is no second owner to clone for.
        if let Some(presenter) = &elems.presenter {
            presenter.set_state(state, Some(slide));
        } else {
            elems.quake.set_slide_cwd(slide.quake_terminal_cwd.clone());
            elems.slide.set_slide(Some(slide), current_step);
        }
    }
}

/// Shows completion toast if presentation is done
fn show_completion_toast_if_done(state: &State, session: &Session) {
    if matches!(state, State::Done { .. }) {
        debug!("Showing done toast");
        session.toast(ToastType::Success, "🎉 Done");
        play_tada();
    }
}

/// Forwards the keyboard's deck commands, unless this client may not drive.
///
/// Checked here rather than in the keyboard service because this is the only
/// place a refusal can be *shown*: a presenter whose laptop registered as
/// audience needs to be told why the arrow keys do nothing, and the server's own
/// refusal would arrive as one error toast per keystroke.
///
/// Only server-bound commands pass through this channel — fullscreen and
/// blanking are handled in the key handler itself — so an audience member keeps
/// the controls that belong to their own screen.
async fn handle_actions(mut rx: UnboundedReceiver<Command>, session: Rc<Session>) {
    while let Some(cmd) = rx.next().await {
        if !session.presents() {
            debug!("Ignoring a command from a client that is not presenting");
            session.toast(ToastType::Info, "This client is watching, not presenting");
            continue;
        }
        if session.tx_cmd.unbounded_send(cmd).is_err() {
            error!("Failed to send command");
        }
    }
}
