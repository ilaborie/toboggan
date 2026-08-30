mod rioterm;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures::channel::mpsc;
use futures::{FutureExt, SinkExt, StreamExt};
use gloo::console::{error, info, warn};
use gloo::net::websocket::Message;
use gloo::net::websocket::futures::WebSocket;
use js_sys::Uint8Array;
use toboggan_core::{TerminalConfig, Theme};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, HtmlElement, KeyboardEvent, Node, ResizeObserver};

use self::rioterm::{CanvasRenderer, OpenOptions, RioTermHandle, RioTheme, Terminal};
use crate::components::WasmElement;
use crate::{
    KeyboardOwner, claim_keyboard, create_and_append_element, create_shadow_root_with_style,
    dom_try, next_keyboard_owner, presenter_token,
};

const CSS: &str = include_str!("style.css");
const DEFAULT_FONT_SIZE: f64 = 22.0;
const FONT_SIZE_STEP: f64 = 2.0;
const FONT_SIZE_MIN: f64 = 8.0;
const FONT_SIZE_MAX: f64 = 32.0;
/// Matches `main.ts`, which preloads these faces before the app starts so the
/// canvas renderer measures cells with the bundled font rather than a fallback.
const FONT_FAMILY: &str = "\"JetBrainsMono Nerd Font Mono\", monospace";

#[derive(Debug)]
pub(crate) struct TobogganTerminalElement {
    container: Option<Element>,
    /// This terminal's identity when it takes the keyboard from the deck.
    ///
    /// Re-minted per session rather than per element, so that a session ending
    /// can give the keyboard back without taking it from whatever claimed it
    /// since. A restart tears the old session down while the new one is already
    /// claiming; matched on the element, that release would land on the wrong
    /// claim.
    owner: Cell<KeyboardOwner>,
    /// The `.terminal-window` of the live session, or `None` before the first
    /// `start_terminal`. Held so the keyboard claim can point at it.
    window: RefCell<Option<HtmlElement>>,
    /// Signals the running session to shut down. `None` when no session is live.
    session: RefCell<Option<mpsc::UnboundedSender<KeyAction>>>,
}

impl Default for TobogganTerminalElement {
    /// Hand-written because the owner id has to come from the shared counter;
    /// `KeyboardOwner` has no meaningful default.
    fn default() -> Self {
        Self {
            container: None,
            owner: Cell::new(next_keyboard_owner()),
            window: RefCell::new(None),
            session: RefCell::new(None),
        }
    }
}

impl TobogganTerminalElement {
    /// Returns `None` if the terminal's DOM could not be built; the caller has
    /// nothing to do about it beyond the logging already done here.
    pub(crate) fn start_terminal(&self, config: &TerminalConfig, api_base_url: &str) -> Option<()> {
        let Some(container) = &self.container else {
            error!("start_terminal called before render");
            return None;
        };

        let document = gloo::utils::document();
        let is_light = config.theme == Theme::Light;

        let window_class = if is_light {
            "terminal-window terminal-light"
        } else {
            "terminal-window terminal-dark"
        };
        let window_el = create_element_with_class(&document, "div", window_class)?;

        let titlebar = create_element_with_class(&document, "div", "terminal-titlebar")?;

        let buttons = create_element_with_class(&document, "div", "terminal-buttons")?;
        let btn_close =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-close")?;
        let btn_minimize =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-minimize")?;
        let btn_maximize =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-maximize")?;
        append_or_log(&buttons, &btn_close, "close button to terminal titlebar");
        append_or_log(
            &buttons,
            &btn_minimize,
            "minimize button to terminal titlebar",
        );
        append_or_log(
            &buttons,
            &btn_maximize,
            "maximize button to terminal titlebar",
        );
        append_or_log(&titlebar, &buttons, "buttons to titlebar");

        let title_text = create_element_with_class(&document, "span", "terminal-title")?;
        let cwd_str = config.cwd.to_string_lossy();
        let title = config
            .cmd
            .as_deref()
            .or_else(|| config.cwd.file_name().and_then(|n| n.to_str()))
            .unwrap_or(&cwd_str);
        title_text.set_text_content(Some(title));
        append_or_log(&titlebar, &title_text, "title text to titlebar");
        append_or_log(&window_el, &titlebar, "titlebar to terminal window");

        // rioterm's `open()` mounts its own container (canvas plus the hidden
        // textarea that takes keystrokes) into whatever element it is handed, so
        // the body is the mount point and there is no canvas to create here.
        let body = create_element_with_class(&document, "div", "terminal-body")?;
        append_or_log(&window_el, &body, "body to terminal window");
        append_or_log(container, &window_el, "terminal window to container");

        let theme = config.theme;
        let title_el = title_text
            .dyn_into::<HtmlElement>()
            .map_err(|_| error!("Failed to cast title element to HtmlElement"))
            .ok();
        let body_el = body
            .dyn_into::<HtmlElement>()
            .map_err(|_| error!("Failed to cast terminal body to HtmlElement"))
            .ok()?;
        let window_html = window_el
            .dyn_into::<HtmlElement>()
            .map_err(|_| error!("Failed to cast window element to HtmlElement"))
            .ok();

        // Set up action channel (shared between font shortcuts, button clicks,
        // the resize observer, and rioterm's own output callback).
        // A fresh identity for a fresh session; see the field's doc.
        self.owner.set(next_keyboard_owner());
        let owner = self.owner.get();

        let (tx_key, rx_key) = mpsc::unbounded::<KeyAction>();
        setup_font_shortcuts(&body_el, tx_key.clone());
        // Re-fit the grid whenever the body's box changes: the very first
        // (post-layout) callback corrects the initial 0-size fallback, and later
        // ones handle window resizes and side-by-side flex reflow.
        setup_resize_observer(&body_el, tx_key.clone());
        setup_button_click(&btn_maximize, tx_key.clone(), KeyAction::Expand);
        setup_button_click(&btn_minimize, tx_key.clone(), KeyAction::Restore);
        if let Some(window_html) = window_html.as_ref() {
            setup_keyboard_claim(window_html, owner, tx_key.clone());
        }
        (*self.window.borrow_mut()).clone_from(&window_html);
        // Kept so `stop_terminal` can end the session (and with it the PTY).
        *self.session.borrow_mut() = Some(tx_key.clone());

        let config = config.clone();
        let api_base_url = api_base_url.to_owned();
        spawn_local(async move {
            run_terminal_session(
                body_el,
                &config,
                &api_base_url,
                theme,
                title_el,
                window_html,
                tx_key,
                rx_key,
                owner,
            )
            .await;
        });
        Some(())
    }

    /// Takes the keyboard for this terminal and focuses its shell.
    ///
    /// The claim is what the deck's key handler reads; the focus is what makes
    /// the keystrokes actually land in the PTY. Both are needed — a claim on its
    /// own would mute the deck without giving the shell anything.
    pub(crate) fn capture_keyboard(&self) {
        let Some(window) = self.window.borrow().clone() else {
            error!("Terminal has no window to claim the keyboard for; the deck keeps its keys");
            return;
        };
        let Some(tx) = self.session.borrow().clone() else {
            error!("Terminal has no live session to focus; the deck keeps its keys");
            return;
        };
        claim_for_live_session(self.owner.get(), &window, &tx);
    }

    /// Gives the deck its keys back.
    pub(crate) fn release_keyboard(&self) {
        crate::release_keyboard(self.owner.get());
    }

    pub(crate) fn stop_terminal(&self) {
        // Before anything else: a terminal that owned the keyboard must not take
        // it to the grave. Advancing off a terminal slide would otherwise leave
        // the deck permanently deaf.
        self.release_keyboard();

        // End the session first: clearing the DOM alone leaves the WebSocket open
        // and the server-side PTY running for the life of the page.
        if let Some(tx) = self.session.borrow_mut().take() {
            // A window that dies while maximized would otherwise leave the
            // registry pointing at a session that can never restore it.
            clear_maximized(&tx);
            let _ = tx.unbounded_send(KeyAction::Shutdown);
        }

        // A maximized window sits in the top layer; removing the element closes
        // the popover implicitly, but saying so keeps the invariant local.
        if let Some(window) = self.window.borrow_mut().take() {
            hide_popover(&window);
        }

        if let Some(container) = &self.container {
            container.set_inner_html("");
        }
    }
}

impl WasmElement for TobogganTerminalElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create shadow root"
        );

        let container: Element = dom_try!(
            create_and_append_element(&root, "div"),
            "create terminal container"
        );
        // Sized explicitly rather than left to inherit the host's layout: an
        // unclassed wrapper is only stretched to the host when the host is a
        // flex container, and any outer rule setting `display` on the host wins
        // over `:host`, so that is not something this component can rely on.
        container.set_class_name("terminal-container");

        self.container = Some(container);
    }
}

/// Creates an element with `class`, or `None` if the DOM refuses.
///
/// Returns an `Option` rather than panicking: a panic here aborts the whole
/// presentation (the crate builds with `panic = "abort"`), and every sibling
/// helper on this path degrades instead.
fn create_element_with_class(
    document: &web_sys::Document,
    tag: &str,
    class: &str,
) -> Option<Element> {
    match document.create_element(tag) {
        Ok(el) => {
            el.set_class_name(class);
            Some(el)
        }
        Err(err) => {
            error!("Failed to create element:", tag, format!("{err:?}"));
            None
        }
    }
}

/// Appends `child` to `parent`, logging `what` on failure instead of panicking.
fn append_or_log(parent: &Element, child: &Node, what: &str) {
    if parent.append_child(child).is_err() {
        error!("Failed to append", what);
    }
}

/// Message from a DOM handler or from rioterm to the session loop.
#[derive(Clone)]
enum KeyAction {
    /// Bytes rioterm wants delivered to the PTY: keystrokes, mouse reports, and
    /// the replies to Device Attributes / cursor-position queries.
    Output(Vec<u8>),
    FontIncrease,
    FontDecrease,
    /// Put the keyboard back in the shell after this terminal took the claim.
    Focus,
    Expand,
    Restore,
    /// The socket closed: end the action loop without touching the screen.
    ///
    /// Deliberately not `Shutdown`: there is nothing left to close, and the
    /// canvas keeps its last frame so the presenter can still read the shell's
    /// parting words. What matters is that the receiver drops, because a send
    /// failing is how a click learns the session is dead.
    SessionEnded,
    /// The terminal body changed size (initial layout, window resize, flex
    /// reflow); re-fit the grid to the new dimensions.
    Resize,
    /// Close the WebSocket and end the session.
    ///
    /// Without this, tearing a terminal down only cleared the DOM: the read loop
    /// stayed parked on the socket, so the server kept the PTY (a shell, two OS
    /// threads, two tokio tasks) alive for the life of the page. Advancing past a
    /// terminal slide and back leaked one PTY per visit.
    Shutdown,
}

/// Observe the terminal body and request a re-fit on every size change.
///
/// The observer (and its closure) are intentionally leaked, matching the
/// keyboard/button handlers: the terminal lives for the slide's lifetime and is
/// torn down by clearing the container, after which the observer stops firing.
fn setup_resize_observer(target: &HtmlElement, tx: mpsc::UnboundedSender<KeyAction>) {
    let closure = Closure::<dyn FnMut()>::new(move || {
        // Session-ended sends fail silently; the observer just stops mattering.
        let _ = tx.unbounded_send(KeyAction::Resize);
    });
    match ResizeObserver::new(closure.as_ref().unchecked_ref()) {
        Ok(observer) => {
            observer.observe(target);
            // Keep the observer alive for the page's lifetime.
            std::mem::forget(observer);
        }
        Err(err) => error!("Failed to create ResizeObserver:", err),
    }
    closure.forget();
}

/// Intercept Cmd+`=` / Cmd+`-` before they reach the terminal.
///
/// Registered in the capture phase on the body, which is an ancestor of the
/// hidden textarea rioterm listens on: stopping propagation here is what keeps
/// the font shortcuts from being encoded and sent to the shell. Every other key
/// falls through to rioterm untouched.
fn setup_font_shortcuts(body: &HtmlElement, tx: mpsc::UnboundedSender<KeyAction>) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: KeyboardEvent| {
        if !event.meta_key() {
            return;
        }
        let action = match event.key().as_str() {
            "=" | "+" => KeyAction::FontIncrease,
            "-" => KeyAction::FontDecrease,
            _ => return,
        };
        event.prevent_default();
        event.stop_propagation();
        let _ = tx.unbounded_send(action);
    });

    if body
        .add_event_listener_with_callback_and_bool(
            "keydown",
            closure.as_ref().unchecked_ref(),
            true,
        )
        .is_err()
    {
        error!("Failed to register terminal font shortcuts");
    }
    closure.forget();
}

thread_local! {
    /// The terminal currently lifted into the top layer, if any.
    ///
    /// One slot, because only one window can be maximized. It exists so the
    /// rest of the deck can take the screen back: the top layer paints above
    /// every z-index there is, so a maximized terminal would otherwise cover
    /// the blanking overlay and the quake terminal alike.
    static MAXIMIZED: RefCell<Option<mpsc::UnboundedSender<KeyAction>>> =
        const { RefCell::new(None) };
}

/// Takes whichever terminal is maximized back out of the top layer.
///
/// Goes through the session channel rather than hiding the popover directly, so
/// the grid is re-fitted to the restored box on the way — a terminal left
/// believing it is full-viewport wraps its shell at the wrong width.
pub(crate) fn restore_maximized_terminal() {
    if let Some(tx) = MAXIMIZED.take() {
        let _ = tx.unbounded_send(KeyAction::Restore);
    }
}

/// Forgets `tx`'s hold on the top layer, if it still has it.
///
/// Identity-matched like the keyboard claim: a restore from a terminal that is
/// not the maximized one must not clear the one that is.
fn clear_maximized(tx: &mpsc::UnboundedSender<KeyAction>) {
    MAXIMIZED.with_borrow_mut(|held| {
        if held.as_ref().is_some_and(|held| held.same_receiver(tx)) {
            *held = None;
        }
    });
}

/// Lifts `window` into the top layer, reporting whether it got there.
///
/// The `popover` attribute is added here rather than when the window is built,
/// because the UA stylesheet dresses *any* `[popover]` — `display: none` until
/// it opens, plus `position: fixed`, `margin: auto`, a border and padding — and
/// that is not a costume the terminal can wear while it is merely sitting in a
/// slide.
///
/// The top layer is what makes this worth doing: it escapes the slide's
/// `overflow`/`transform` clipping *without the element leaving the tree*, so
/// every selector that styles it from the outside keeps matching. Moving the
/// shadow host to `<body>`, which is how this used to work, silently undressed
/// the quake terminal — its chrome and sizing come from
/// `.toboggan-quake-terminal > .toboggan-quake-inner`.
fn show_as_popover(window: &HtmlElement) -> bool {
    if window.set_attribute("popover", "manual").is_err() {
        error!("Failed to make the terminal window a popover");
        return false;
    }
    if let Err(err) = window.show_popover() {
        error!("Failed to maximize the terminal:", format!("{err:?}"));
        let _ = window.remove_attribute("popover");
        return false;
    }
    true
}

/// Returns `window` to the flow, dropping the attribute so the UA popover rules
/// stop applying to it.
///
/// Both calls are allowed to fail: they do so when the window was never
/// maximized, which is the state being asked for.
fn hide_popover(window: &HtmlElement) {
    let _ = window.hide_popover();
    let _ = window.remove_attribute("popover");
}

/// Takes the keyboard for `window`, but only if its session can still take a
/// keystroke.
///
/// The claim and the focus are one decision, not two steps. A claim mutes the
/// deck, so granting one to a session whose loop has already exited leaves the
/// presenter clicking a dead terminal that shows nothing, wearing the ring that
/// says it has the keys, while the deck stops answering. `unbounded_send` fails
/// exactly when that loop is gone — the socket closed, the server restarted, or
/// the shell was `exit`ed — which makes it the test.
fn claim_for_live_session(
    owner: KeyboardOwner,
    window: &HtmlElement,
    tx: &mpsc::UnboundedSender<KeyAction>,
) {
    if tx.unbounded_send(KeyAction::Focus).is_err() {
        warn!("Terminal session has ended; leaving the keyboard with the deck");
        crate::release_keyboard(owner);
        return;
    }
    claim_keyboard(owner, window);
}

/// Takes the keyboard whenever a click lands anywhere in the terminal window.
///
/// Capture-phase `mousedown` on the window rather than relying on focus: rioterm
/// focuses its hidden textarea only from a `mousedown` on its own canvas
/// container, so the title bar, the traffic lights and the body padding all left
/// the terminal looking active while the deck still answered `space`.
///
/// Deliberately no `prevent_default`: rioterm relies on the browser's own
/// `mousedown` defaults for text selection and `prevent_default` would suppress
/// them. Propagation is untouched either way — [`KeyAction::Focus`] crosses a
/// channel, so it lands after those defaults have run, and focus ends up in the
/// shell regardless.
fn setup_keyboard_claim(
    window: &HtmlElement,
    owner: KeyboardOwner,
    tx: mpsc::UnboundedSender<KeyAction>,
) {
    let window_for_claim = window.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
        claim_for_live_session(owner, &window_for_claim, &tx);
    });
    if window
        .add_event_listener_with_callback_and_bool(
            "mousedown",
            closure.as_ref().unchecked_ref(),
            true,
        )
        .is_err()
    {
        error!("Failed to register terminal keyboard claim");
    }
    closure.forget();
}

fn setup_button_click(btn: &Element, tx: mpsc::UnboundedSender<KeyAction>, action: KeyAction) {
    let closure = Closure::<dyn FnMut()>::new(move || {
        if tx.unbounded_send(action.clone()).is_err() {
            // Session has ended; handlers are still registered but input is dropped.
        }
    });
    if btn
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .is_err()
    {
        error!("Failed to register button click handler");
    }
    closure.forget();
}

/// A live rioterm instance plus the callbacks it holds.
///
/// The closures are fields rather than `forget()`ed because a font-size change
/// disposes the whole terminal and builds a new one; leaking a pair per keypress
/// would accumulate for the life of the page.
struct Session {
    handle: RioTermHandle,
    terminal: Terminal,
    renderer: CanvasRenderer,
    _on_data: Closure<dyn FnMut(Uint8Array)>,
    _on_title: Closure<dyn FnMut(String)>,
}

/// Body horizontal padding: left 3px + right 3px (CSS: .terminal-body { padding: 2px 3px 3px })
const BODY_H_PADDING: f64 = 6.0;
/// Body vertical padding: top 2px + bottom 3px (CSS: .terminal-body { padding: 2px 3px 3px })
const BODY_V_PADDING: f64 = 5.0;
/// Stand-in grid for a body that cannot be measured yet.
const DEFAULT_GRID: (u16, u16) = (80, 24);

/// The custom property a host sets to open its terminals at another size.
const FONT_SIZE_PROPERTY: &str = "--terminal-font-size";
/// Below half a CSS pixel two font sizes almost always give the same grid — the
/// cell is a whole number of pixels, near `.6` by `1.3` of the size — so a
/// rebuild would replay the buffer for a difference nobody can see.
const FONT_SIZE_EPSILON: f64 = 0.5;

/// The font size the host asked for, in CSS pixels, or `None` for "not asked".
///
/// Custom properties are the only thing that crosses into a component's shadow
/// tree from outside, and this is the third one the terminal honours, after
/// `--terminal-radius` and `--terminal-titlebar-display`. It is *read* rather
/// than applied because rioterm fixes `fontSize` when the renderer is built —
/// see [`rebuild_for_font_size`] for what changing it afterwards costs.
///
/// Read off `.terminal-body`, which inherits it: a slide can therefore declare
/// it on the terminal element, or anywhere above it, exactly as it declares
/// `--terminal-radius`.
///
/// See [`parse_font_size`] for what counts as an answer.
///
/// A value that was declared and refused is warned about here rather than in the
/// parser: a deck that writes `2em` and gets 22 with no explanation would look
/// for the fault in its selector for a long time. The parser stays free of
/// `gloo::console`, which is what lets it be tested on the host target.
fn css_font_size(body: &HtmlElement) -> Option<f64> {
    let style = web_sys::window()?.get_computed_style(body).ok()??;
    let declared = style.get_property_value(FONT_SIZE_PROPERTY).ok()?;
    let size = parse_font_size(&declared);
    if size.is_none() && !declared.trim().is_empty() {
        warn!(
            "Ignoring --terminal-font-size (want a number of CSS pixels):",
            declared
        );
    }
    size
}

/// A declared `--terminal-font-size`, in CSS pixels, or `None` for "not asked".
///
/// Unitless or `px`; anything else (`em`, `2vw`, a typo, the empty string a
/// computed style gives for a property nobody set) is ignored rather than
/// guessed at — the value has to reach rioterm as a number, and no element is
/// available to resolve a relative unit against.
///
/// Clamped to the same 8–32 the keyboard is bounded by, so a stray `260` opens
/// a readable terminal rather than a two-column one, on stage, with no way back
/// but `Cmd -`.
///
/// Pure, and deliberately so: it is the one part of this module the host-target
/// unit tests can reach. Logging lives in [`css_font_size`].
fn parse_font_size(declared: &str) -> Option<f64> {
    let declared = declared.trim();
    let number = declared.strip_suffix("px").unwrap_or(declared).trim();
    let size = number.parse::<f64>().ok()?;
    // `clamp` panics on a NaN bound and would hand `inf` back as FONT_SIZE_MAX,
    // which is a size nobody asked for.
    size.is_finite()
        .then(|| size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX))
}

/// Mounts a rioterm terminal into `body` and wires its callbacks to `tx`.
/// Loads the bundled terminal font, so the canvas measures cell width against
/// the real font rather than a system fallback.
///
/// Calls the page's memoised starter (`window.tobogganFontsReady`) rather than
/// waiting on `document.fonts.ready`: that one settles only when the document
/// has no font work left at all, which on a deck that keeps swapping slides can
/// take a long time — long enough to stall a terminal waiting on four faces.
/// This is also what pulls the faces down in the first place, so the ~700 KB is
/// charged to the terminal that needs it and not to the first slide.
///
/// Returns immediately when the starter is absent — the presenter view has no
/// terminals and never publishes one.
async fn await_terminal_fonts() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(starter) = js_sys::Reflect::get(&window, &JsValue::from_str("tobogganFontsReady"))
    else {
        return;
    };
    let Ok(starter) = starter.dyn_into::<js_sys::Function>() else {
        return;
    };
    let Ok(promise) = starter.call0(&JsValue::NULL) else {
        warn!("Could not start the terminal font load");
        return;
    };
    let Ok(promise) = promise.dyn_into::<js_sys::Promise>() else {
        return;
    };
    if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
        warn!("Terminal fonts did not finish loading:", err);
    }
}

async fn open_session(
    body: &HtmlElement,
    theme: Theme,
    font_size: f64,
    title_el: Option<HtmlElement>,
    tx: &mpsc::UnboundedSender<KeyAction>,
) -> Option<Session> {
    // The canvas renderer measures cell width once, when it is constructed, and
    // falls back to a system font if the bundled one has not arrived — box
    // drawing and powerline glyphs then sit out of line for the whole session.
    // Waited for here, at the one place a terminal is built, rather than in
    // front of `start_app`: gating the whole deck on a 700 KB font download put
    // a wait only terminals need in front of the first slide.
    await_terminal_fonts().await;

    let options = OpenOptions {
        renderer: "canvas",
        // We drive fitting ourselves: rioterm's own observer would resize the
        // grid without telling us, and the server's PTY has to learn the new
        // size or the shell keeps drawing to the old one.
        fit: false,
        auto_focus: true,
        // Off because it is broken upstream in rioterm 0.1.8, not because we do
        // not want it. `open()` registers its own `onData` listener that calls
        // `PredictionEngine.onInput` -> `Terminal.cursorPosition()`, and those
        // listeners fire synchronously from inside `Terminal.key()` — so the
        // second call re-enters a wasm object the first still holds, and
        // wasm-bindgen rejects it: "recursive use of an object detected which
        // would lead to unsafe aliasing in rust". rioterm catches the throw, so
        // input still reaches the shell, but nothing is ever predicted and every
        // keystroke logs an error — one per character, during a live talk.
        // Flip this back once upstream defers the prediction hop.
        predictive_echo: false,
        font_family: FONT_FAMILY.to_owned(),
        font_size,
        theme: if theme == Theme::Light {
            RioTheme::LIGHT
        } else {
            RioTheme::DARK
        },
    };
    let options = match serde_wasm_bindgen::to_value(&options) {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to serialize terminal options:", err.to_string());
            return None;
        }
    };

    let handle = match rioterm::open(body, &options).await {
        Ok(handle) => handle.unchecked_into::<RioTermHandle>(),
        Err(err) => {
            error!("Failed to open the terminal:", format!("{err:?}"));
            return None;
        }
    };
    let terminal = handle.terminal();
    let renderer = handle.renderer();

    // rioterm mounts its own canvas; give it the deck's canvas styling so the
    // stylesheet keeps applying without knowing who created the element.
    renderer.element().set_class_name("terminal-canvas");

    let tx_data = tx.clone();
    let on_data = Closure::<dyn FnMut(Uint8Array)>::new(move |bytes: Uint8Array| {
        let _ = tx_data.unbounded_send(KeyAction::Output(bytes.to_vec()));
    });
    terminal.on_data(on_data.as_ref());

    let on_title = Closure::<dyn FnMut(String)>::new(move |title: String| {
        if let Some(el) = title_el.as_ref()
            && !title.is_empty()
        {
            el.set_text_content(Some(&title));
        }
    });
    terminal.on_title_change(on_title.as_ref());

    Some(Session {
        handle,
        terminal,
        renderer,
        _on_data: on_data,
        _on_title: on_title,
    })
}

/// The write half of the terminal's socket, shared by every sender.
type WsWrite = Rc<futures::lock::Mutex<futures::stream::SplitSink<WebSocket, Message>>>;

/// Re-fits the grid to the body's content box and returns the size it settled
/// on, or `None` when that box is empty — `client_width`/`client_height` at or
/// below the padding, which is what a body measured before layout gives.
///
/// rioterm's own `fit` silently no-ops on a box it cannot draw a cell in.
/// Reporting the unchanged grid as the settled one would then tell the server
/// that a stale size is current — so the caller is left holding the previous
/// dimensions instead, and the `ResizeObserver` drives the real fit
/// once layout has caught up.
fn refit(session: &Session, body: &HtmlElement) -> Option<(u16, u16)> {
    let width = f64::from(body.client_width()) - BODY_H_PADDING;
    let height = f64::from(body.client_height()) - BODY_V_PADDING;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    session.renderer.fit(width, height);
    let options = session.terminal.options();
    Some((options.cols(), options.rows()))
}

/// Re-fits and tells the server, returning the dimensions now in force.
///
/// Takes the cell rather than a borrow so the `RefCell` is released before the
/// socket is awaited.
async fn refit_and_send(
    session: &Rc<RefCell<Session>>,
    body: &HtmlElement,
    ws_write: &WsWrite,
    last: (u16, u16),
) -> (u16, u16) {
    let dims = refit(&session.borrow(), body);
    let Some(dims) = dims else {
        return last;
    };
    send_resize_if_changed(ws_write, dims, last).await
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn run_terminal_session(
    body: HtmlElement,
    config: &TerminalConfig,
    api_base_url: &str,
    theme: Theme,
    title_el: Option<HtmlElement>,
    window_el: Option<HtmlElement>,
    tx_key: mpsc::UnboundedSender<KeyAction>,
    rx_key: mpsc::UnboundedReceiver<KeyAction>,
    owner: KeyboardOwner,
) {
    // A deck that declares nothing opens at DEFAULT_FONT_SIZE; one that declares
    // `--terminal-font-size` opens at what it asked for. The `Resize` arm below
    // is where a stylesheet that had not arrived yet gets its second chance.
    let initial_font_size = css_font_size(&body).unwrap_or(DEFAULT_FONT_SIZE);
    let font_size = Rc::new(RefCell::new(initial_font_size));
    let Some(session) =
        open_session(&body, theme, initial_font_size, title_el.clone(), &tx_key).await
    else {
        return;
    };

    // Size the grid before connecting: the PTY is spawned with the dimensions in
    // the URL, so getting them right here saves the shell an initial redraw.
    // Pre-layout the body can still measure zero, and the ResizeObserver's first
    // callback corrects that — so a conventional 80x24 stands in until it does.
    let (initial_cols, initial_rows) = refit(&session, &body).unwrap_or(DEFAULT_GRID);
    let ws_url = build_terminal_ws_url(api_base_url, config, initial_cols, initial_rows);
    // The URL ends in `&token=…` when one was offered, so it is logged without
    // its query string: this ran on every terminal open.
    let logged_url = ws_url.split('?').next().unwrap_or(&ws_url);
    info!("Starting terminal session:", logged_url);

    let ws = match WebSocket::open(&ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            error!("Failed to connect to terminal:", err.to_string());
            session.handle.dispose();
            return;
        }
    };

    let session = Rc::new(RefCell::new(session));
    let (ws_write, mut ws_read) = ws.split();
    // Signals the read loop to stop. Closing the sink is not enough: the split
    // halves share the underlying socket, so while the read half is still parked
    // on `next()` the socket stays open and the server keeps the PTY alive.
    let (tx_stop, rx_stop) = futures::channel::oneshot::channel::<()>();

    let ws_write = Rc::new(futures::lock::Mutex::new(ws_write));
    let ws_write_action = Rc::clone(&ws_write);
    let session_action = Rc::clone(&session);
    let body_action = body.clone();
    let window_el_action = window_el.clone();
    let font_size_action = Rc::clone(&font_size);
    let tx_key_end = tx_key.clone();

    spawn_local(async move {
        let mut rx_key = rx_key;
        let mut tx_stop = Some(tx_stop);
        // Last grid size sent to the server, so a re-fit that resolves to the
        // same dimensions (a spurious observer callback) is a no-op.
        let mut last_dims = (initial_cols, initial_rows);
        // Whether `--terminal-font-size` has had its last word. See the `Resize`
        // arm: the first re-fit is the deadline, and after it the size belongs
        // to whoever last pressed `Cmd +`.
        let mut css_size_settled = false;
        while let Some(action) = rx_key.next().await {
            match action {
                KeyAction::Shutdown => {
                    let _ = ws_write_action.lock().await.close().await;
                    // Wake the read loop so both halves drop and the socket
                    // actually closes.
                    if let Some(tx) = tx_stop.take() {
                        let _ = tx.send(());
                    }
                    session_action.borrow().handle.dispose();
                    break;
                }
                KeyAction::Output(bytes) => {
                    // Binary is PTY input; the resize command goes as text, so
                    // the server never has to guess which one it received.
                    let sent = ws_write_action
                        .lock()
                        .await
                        .send(Message::Bytes(bytes))
                        .await;
                    if sent.is_err() {
                        break;
                    }
                }
                KeyAction::FontIncrease | KeyAction::FontDecrease => {
                    let new_size = {
                        let mut size = font_size_action.borrow_mut();
                        *size = if matches!(action, KeyAction::FontIncrease) {
                            (*size + FONT_SIZE_STEP).min(FONT_SIZE_MAX)
                        } else {
                            (*size - FONT_SIZE_STEP).max(FONT_SIZE_MIN)
                        };
                        *size
                    };
                    if let Some(dims) = rebuild_for_font_size(
                        &session_action,
                        &body_action,
                        theme,
                        new_size,
                        title_el.clone(),
                        &tx_key,
                    )
                    .await
                    {
                        last_dims = send_resize_if_changed(&ws_write_action, dims, last_dims).await;
                    }
                }
                KeyAction::Focus => session_action.borrow().handle.focus(),
                KeyAction::Expand => {
                    if let Some(ref win) = window_el_action
                        && !show_as_popover(win)
                    {
                        // Still fixed and full-viewport, just back to being
                        // clipped by whatever the slide does.
                        if win.class_list().add_1("terminal-fullscreen").is_err() {
                            error!("Failed to add terminal-fullscreen class on expand");
                        }
                    }
                    MAXIMIZED.set(Some(tx_key.clone()));
                    last_dims =
                        refit_and_send(&session_action, &body_action, &ws_write_action, last_dims)
                            .await;
                    session_action.borrow().handle.focus();
                }
                KeyAction::Restore => {
                    if let Some(ref win) = window_el_action {
                        hide_popover(win);
                        if win.class_list().remove_1("terminal-fullscreen").is_err() {
                            error!("Failed to remove terminal-fullscreen class on restore");
                        }
                    }
                    clear_maximized(&tx_key);
                    last_dims =
                        refit_and_send(&session_action, &body_action, &ws_write_action, last_dims)
                            .await;
                    session_action.borrow().handle.focus();
                }
                KeyAction::Resize => {
                    // A slide's stylesheet can still be loading when its
                    // terminals open — an `@import` resolves asynchronously, and
                    // the deck's own CSS usually sits behind one — so the size
                    // read before `open_session` may have been read too early.
                    // The first re-fit is the deadline: by then the observer has
                    // seen a laid-out body, which means styles are resolved.
                    //
                    // Once, and only from the *initial* size: after this the
                    // presenter may have pressed `Cmd +`, and a stylesheet does
                    // not get to argue with the person on stage.
                    let late = if css_size_settled {
                        None
                    } else {
                        css_size_settled = true;
                        let in_force = *font_size_action.borrow();
                        let untouched = (in_force - initial_font_size).abs() < FONT_SIZE_EPSILON;
                        css_font_size(&body_action)
                            .filter(|_| untouched)
                            .filter(|wanted| (wanted - in_force).abs() >= FONT_SIZE_EPSILON)
                    };
                    let rebuilt = match late {
                        Some(wanted) => {
                            *font_size_action.borrow_mut() = wanted;
                            rebuild_for_font_size(
                                &session_action,
                                &body_action,
                                theme,
                                wanted,
                                title_el.clone(),
                                &tx_key,
                            )
                            .await
                        }
                        None => None,
                    };
                    last_dims = match rebuilt {
                        Some(dims) => {
                            send_resize_if_changed(&ws_write_action, dims, last_dims).await
                        }
                        None => {
                            refit_and_send(
                                &session_action,
                                &body_action,
                                &ws_write_action,
                                last_dims,
                            )
                            .await
                        }
                    };
                }
                KeyAction::SessionEnded => break,
            }
        }
    });

    // Read terminal output from the server and hand it to the emulator, which
    // repaints itself.
    let mut rx_stop = rx_stop.fuse();
    loop {
        let msg = futures::select! {
            msg = ws_read.next().fuse() => match msg {
                Some(msg) => msg,
                None => break,
            },
            _ = rx_stop => break,
        };
        match msg {
            Ok(Message::Bytes(data)) => session.borrow().terminal.write(&data),
            Ok(Message::Text(text)) => session.borrow().terminal.write(text.as_bytes()),
            Err(err) => {
                error!("Terminal WebSocket error:", err.to_string());
                break;
            }
        }
    }

    info!("Terminal session ended");
    // The action loop is still parked on its channel, and would go on accepting
    // work for a session that has nothing behind it — including a claim on the
    // keyboard, which would mute the deck for a terminal that cannot take a
    // keystroke. Ending it makes the send fail, which is what tells a later
    // click to leave the keys alone.
    clear_maximized(&tx_key_end);
    let _ = tx_key_end.unbounded_send(KeyAction::SessionEnded);
    // And the keyboard goes back to the deck, so a shell that exits does not
    // leave the presenter holding a ring over a dead terminal. Safe from here
    // because the owner names this session: a restart's teardown releases the id
    // the old session claimed with, which no longer matches the one the new
    // session already holds, so it is a no-op rather than a theft.
    crate::release_keyboard(owner);
    let _ = ws_write.lock().await.close().await;
    // Drop both halves so the underlying socket is released and the server sees
    // the close.
    drop(ws_read);
}

/// Rebuilds the terminal at a new font size, carrying the screen across.
///
/// rioterm fixes `fontSize` when the renderer is constructed, and `open()` wires
/// its pointer, wheel and clipboard handlers to that one renderer — swapping the
/// renderer alone would leave those closures pointing at the disposed one. So the
/// whole terminal is rebuilt and the buffer replayed through `serialize()`, which
/// is the documented way to reproduce content, styling and links in a fresh
/// terminal. Returns the new grid size, or `None` if the rebuild failed (the old
/// terminal is gone by then, so the session ends).
async fn rebuild_for_font_size(
    session: &Rc<RefCell<Session>>,
    body: &HtmlElement,
    theme: Theme,
    font_size: f64,
    title_el: Option<HtmlElement>,
    tx: &mpsc::UnboundedSender<KeyAction>,
) -> Option<(u16, u16)> {
    // Take the snapshot and drop the borrow before awaiting.
    let dump = {
        let current = session.borrow();
        let dump = current.terminal.serialize();
        current.handle.dispose();
        dump
    };

    let next = open_session(body, theme, font_size, title_el, tx).await?;
    next.terminal.write(dump.as_bytes());
    let dims = refit(&next, body);
    *session.borrow_mut() = next;
    dims
}

/// Tells the server to resize the PTY when the grid actually changed, and
/// returns the dimensions now in force.
async fn send_resize_if_changed(
    ws_write: &WsWrite,
    dims: (u16, u16),
    last: (u16, u16),
) -> (u16, u16) {
    if dims == last {
        return last;
    }
    let (cols, rows) = dims;
    let resize = format!(r#"{{"type":"resize","cols":{cols},"rows":{rows}}}"#);
    if ws_write
        .lock()
        .await
        .send(Message::Text(resize))
        .await
        .is_err()
    {
        error!("Failed to send resize to server");
    }
    dims
}

fn build_terminal_ws_url(
    api_base_url: &str,
    config: &TerminalConfig,
    cols: u16,
    rows: u16,
) -> String {
    let ws_base = api_base_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    let cwd_str = config.cwd.to_string_lossy();
    let encoded_cwd = String::from(js_sys::encode_uri_component(&cwd_str));
    let mut url = format!("{ws_base}/api/terminal?cwd={encoded_cwd}&cols={cols}&rows={rows}");

    if let Some(cmd) = &config.cmd {
        url.push_str("&cmd=");
        url.push_str(&String::from(js_sys::encode_uri_component(cmd)));
    }

    // Opening a shell is presenter-only, and a browser cannot put a header on a
    // WebSocket — so the token travels here, in the same query string as the
    // rest of the session's parameters.
    if let Some(token) = presenter_token() {
        url.push_str("&token=");
        url.push_str(&String::from(js_sys::encode_uri_component(token.expose())));
    }

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_number_is_a_font_size() {
        assert_eq!(parse_font_size("26"), Some(26.0));
        // A computed style hands the value back with the author's whitespace.
        assert_eq!(parse_font_size(" 26 "), Some(26.0));
    }

    #[test]
    fn px_is_tolerated_because_it_is_what_a_reflex_types() {
        assert_eq!(parse_font_size("26px"), Some(26.0));
        assert_eq!(parse_font_size(" 26 px "), Some(26.0));
    }

    #[test]
    fn nothing_declared_is_not_a_font_size() {
        // What `getPropertyValue` returns for a property no rule sets.
        assert_eq!(parse_font_size(""), None);
        assert_eq!(parse_font_size("   "), None);
    }

    #[test]
    fn a_relative_unit_is_refused_rather_than_guessed_at() {
        // There is no element to resolve these against by the time rioterm
        // wants a number, so they fall back to DEFAULT_FONT_SIZE.
        assert_eq!(parse_font_size("2em"), None);
        assert_eq!(parse_font_size("2vw"), None);
        assert_eq!(parse_font_size("large"), None);
    }

    #[test]
    fn a_typo_cannot_open_an_unusable_terminal() {
        assert_eq!(parse_font_size("260"), Some(FONT_SIZE_MAX));
        assert_eq!(parse_font_size("2"), Some(FONT_SIZE_MIN));
        assert_eq!(parse_font_size("-26"), Some(FONT_SIZE_MIN));
    }

    #[test]
    fn infinity_is_not_clamped_into_a_size() {
        // `clamp` would happily return 32 for `inf`, and NaN panics in it.
        assert_eq!(parse_font_size("inf"), None);
        assert_eq!(parse_font_size("NaN"), None);
    }
}
