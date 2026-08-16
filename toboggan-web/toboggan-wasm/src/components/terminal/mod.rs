mod vterm;

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc;
use futures::{FutureExt, SinkExt, StreamExt};
use gloo::console::{error, info};
use gloo::net::websocket::Message;
use gloo::net::websocket::futures::WebSocket;
use toboggan_core::{TerminalConfig, Theme};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    Element, HtmlCanvasElement, HtmlElement, KeyboardEvent, Node, ResizeObserver, ShadowRoot,
};

use self::vterm::VirtualTerminal;
use crate::components::WasmElement;
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try};

const CSS: &str = include_str!("style.css");
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const DEFAULT_FONT_SIZE: f64 = 22.0;
const FONT_SIZE_STEP: f64 = 2.0;
const FONT_SIZE_MIN: f64 = 8.0;
const FONT_SIZE_MAX: f64 = 32.0;
/// Smallest grid the terminal is ever sized to, so a collapsed box still yields a usable PTY.
const MIN_COLS: u16 = 20;
const MIN_ROWS: u16 = 4;

#[derive(Debug, Default)]
pub(crate) struct TobogganTerminalElement {
    container: Option<Element>,
    /// Signals the running session to shut down. `None` when no session is live.
    session: RefCell<Option<mpsc::UnboundedSender<KeyAction>>>,
    /// Whether this terminal's shadow host outlives its sessions.
    ///
    /// Set for the quake overlay, which creates one host at render time and
    /// re-populates it on every restart — removing that host would leave the
    /// overlay permanently empty.
    persistent: bool,
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

        let body = create_element_with_class(&document, "div", "terminal-body")?;

        let canvas = create_terminal_canvas(&document)?;

        append_or_log(&body, &canvas, "canvas to terminal body");
        append_or_log(&window_el, &body, "body to terminal window");
        append_or_log(container, &window_el, "terminal window to container");

        // focus() failure is non-critical (e.g. element not yet visible)
        let _ = canvas.focus();

        let theme = config.theme;
        let title_el = title_text
            .dyn_into::<HtmlElement>()
            .map_err(|_| error!("Failed to cast title element to HtmlElement"))
            .ok();
        let window_html = window_el
            .dyn_into::<HtmlElement>()
            .map_err(|_| error!("Failed to cast window element to HtmlElement"))
            .ok();
        // Both dimensions, not just rows: passing the computed rows alongside a
        // hardcoded 80 columns made every session's first frame 80 wide no matter
        // how wide the pane was, until the first resize corrected it.
        let (initial_cols, initial_rows) =
            compute_terminal_size(window_html.as_ref(), DEFAULT_FONT_SIZE);
        let ws_url = build_terminal_ws_url(api_base_url, config, initial_cols, initial_rows);

        // Set up action channel (shared between keyboard handler and button clicks)
        let (tx_key, rx_key) = mpsc::unbounded::<KeyAction>();
        setup_keyboard_handler(&canvas, tx_key.clone());
        // Re-fit the grid whenever the body's box changes: the very first
        // (post-layout) callback corrects the initial 0-size fallback, and later
        // ones handle window resizes and side-by-side flex reflow.
        setup_resize_observer(&body, tx_key.clone());
        setup_button_click(&btn_maximize, tx_key.clone(), KeyAction::Expand);
        setup_button_click(&btn_minimize, tx_key.clone(), KeyAction::Restore);
        // Kept so `stop_terminal` can end the session (and with it the PTY).
        *self.session.borrow_mut() = Some(tx_key);

        info!("Starting terminal session:", &ws_url);

        spawn_local(async move {
            run_terminal_session(
                canvas,
                &ws_url,
                theme,
                title_el,
                window_html,
                (initial_cols, initial_rows),
                rx_key,
            )
            .await;
        });
        Some(())
    }

    pub(crate) fn stop_terminal(&self) {
        // End the session first: clearing the DOM alone leaves the WebSocket open
        // and the server-side PTY running for the life of the page.
        if let Some(tx) = self.session.borrow_mut().take() {
            let _ = tx.unbounded_send(KeyAction::Shutdown);
        }

        let Some(container) = &self.container else {
            return;
        };
        container.set_inner_html("");
        // A fullscreen terminal's shadow host is relocated to <body> (so
        // `position: fixed` escapes the slide's clipping). Clearing the slide's
        // own container never touches it, so such a host would keep floating over
        // later slides — remove it directly.
        //
        // Never for a persistent host: `Expand` moves *any* host to <body>, so a
        // fullscreen quake terminal is a body child too, and removing it would
        // detach the overlay's one reusable host for good.
        if self.persistent {
            return;
        }
        if let Ok(root) = container.get_root_node().dyn_into::<ShadowRoot>() {
            let host = root.host();
            if is_child_of_body(&host) {
                host.remove();
            }
        }
    }

    /// Marks this terminal's shadow host as reused across sessions.
    pub(crate) fn set_persistent(&mut self, persistent: bool) {
        self.persistent = persistent;
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

/// Creates the focusable `<canvas>` the terminal renders into, or logs and
/// returns `None` if the element cannot be created.
fn create_terminal_canvas(document: &web_sys::Document) -> Option<HtmlCanvasElement> {
    let Ok(element) = document.create_element("canvas") else {
        error!("Failed to create canvas element");
        return None;
    };
    let Ok(canvas) = element.dyn_into::<HtmlCanvasElement>() else {
        error!("Failed to cast to HtmlCanvasElement");
        return None;
    };
    canvas.set_class_name("terminal-canvas");
    if canvas.set_attribute("tabindex", "0").is_err() {
        error!("Failed to make terminal canvas focusable");
    }
    Some(canvas)
}

/// Whether `el` is a direct child of `<body>`, i.e. a terminal host that was lifted
/// out of its slide for fullscreen and never restored.
///
/// Callers must exclude persistent hosts themselves: `KeyAction::Expand` moves
/// whichever host it is given to `<body>`, so this matches an expanded quake host
/// as readily as a slide's.
fn is_child_of_body(el: &Element) -> bool {
    let Some(parent) = el.parent_node() else {
        return false;
    };
    gloo::utils::document()
        .body()
        .is_some_and(|body| body.is_same_node(Some(&parent)))
}

/// Resolve the shadow-DOM host element for a node living inside the terminal's shadow root.
///
/// Used to lift the whole terminal (host + its scoped styles) out of the slide on
/// fullscreen, since the slide's `overflow`/`transform` would otherwise clip or trap the
/// `position: fixed` terminal.
fn shadow_host(el: &HtmlElement) -> Option<Element> {
    el.get_root_node()
        .dyn_into::<ShadowRoot>()
        .ok()
        .map(|root| root.host())
}

/// Message from keyboard/button/resize handler to terminal session
#[derive(Clone)]
enum KeyAction {
    Input(String),
    FontIncrease,
    FontDecrease,
    Expand,
    Restore,
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
fn setup_resize_observer(target: &Element, tx: mpsc::UnboundedSender<KeyAction>) {
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

#[allow(clippy::too_many_lines)]
async fn run_terminal_session(
    canvas: HtmlCanvasElement,
    ws_url: &str,
    theme: Theme,
    title_el: Option<HtmlElement>,
    window_el: Option<HtmlElement>,
    // The grid the session opens with, as `(cols, rows)`.
    initial_size: (u16, u16),
    rx_key: mpsc::UnboundedReceiver<KeyAction>,
) {
    let (initial_cols, initial_rows) = initial_size;
    let ws = match WebSocket::open(ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            error!("Failed to connect to terminal:", err.to_string());
            return;
        }
    };

    let (ws_write, mut ws_read) = ws.split();
    // Signals the read loop to stop. Closing the sink is not enough: the split
    // halves share the underlying socket, so while the read half is still parked
    // on `next()` the socket stays open and the server keeps the PTY alive.
    let (tx_stop, rx_stop) = futures::channel::oneshot::channel::<()>();
    let font_size = Rc::new(RefCell::new(DEFAULT_FONT_SIZE));

    let vterm = VirtualTerminal::new(initial_cols, initial_rows, theme);

    vterm.render_to_canvas(&canvas, *font_size.borrow());

    // Forward actions (keyboard input, font resize, expand/restore) to WebSocket
    let ws_write = Rc::new(futures::lock::Mutex::new(ws_write));
    let ws_write_kbd = Rc::clone(&ws_write);
    let font_size_kbd = Rc::clone(&font_size);
    let canvas_kbd = canvas.clone();
    let vterm_rc = Rc::new(RefCell::new(vterm));
    let vterm_kbd = Rc::clone(&vterm_rc);
    let window_el_kbd = window_el.clone();

    spawn_local(async move {
        let mut rx_key = rx_key;
        let mut tx_stop = Some(tx_stop);
        // The terminal's styles live in a shadow root, so on fullscreen we move the whole
        // host (not the inner window) to `<body>` to escape the slide's clipping/transform,
        // then restore it to its original position on collapse.
        let host_el = window_el_kbd.as_ref().and_then(shadow_host);
        let mut fullscreen_origin: Option<(Node, Option<Node>)> = None;
        // Last grid size sent to the server, so a `Resize` that resolves to the
        // same dimensions (a spurious observer callback) is a no-op.
        let mut last_dims = (initial_cols, initial_rows);
        while let Some(action) = rx_key.next().await {
            match action {
                KeyAction::Shutdown => {
                    let _ = ws_write_kbd.lock().await.close().await;
                    // Wake the read loop so both halves drop and the socket
                    // actually closes.
                    if let Some(tx) = tx_stop.take() {
                        let _ = tx.send(());
                    }
                    break;
                }
                KeyAction::Input(input) => {
                    let send_result = ws_write_kbd.lock().await.send(Message::Text(input)).await;
                    if send_result.is_err() {
                        break;
                    }
                }
                KeyAction::FontIncrease | KeyAction::FontDecrease => {
                    let new_size = {
                        let mut size = font_size_kbd.borrow_mut();
                        *size = if matches!(action, KeyAction::FontIncrease) {
                            (*size + FONT_SIZE_STEP).min(FONT_SIZE_MAX)
                        } else {
                            (*size - FONT_SIZE_STEP).max(FONT_SIZE_MIN)
                        };
                        *size
                    };
                    last_dims = refit(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        window_el_kbd.as_ref(),
                        new_size,
                    )
                    .await;
                }
                KeyAction::Expand => {
                    if let Some(ref win) = window_el_kbd
                        && win.class_list().add_1("terminal-fullscreen").is_err()
                    {
                        error!("Failed to add terminal-fullscreen class on expand");
                    }
                    // Lift the host out of the slide so `position: fixed` resolves against
                    // the viewport (the slide's overflow/transform would otherwise clip it).
                    if let Some(ref host) = host_el {
                        if fullscreen_origin.is_none()
                            && let Some(parent) = host.parent_node()
                        {
                            fullscreen_origin = Some((parent, host.next_sibling()));
                        }
                        if let Some(body) = gloo::utils::document().body()
                            && body.append_child(host).is_err()
                        {
                            error!("Failed to move terminal host to <body> on expand");
                        }
                    }
                    let size = *font_size_kbd.borrow();
                    last_dims = refit(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        window_el_kbd.as_ref(),
                        size,
                    )
                    .await;
                    let _ = canvas_kbd.focus();
                }
                KeyAction::Restore => {
                    if let Some(ref win) = window_el_kbd
                        && win.class_list().remove_1("terminal-fullscreen").is_err()
                    {
                        error!("Failed to remove terminal-fullscreen class on restore");
                    }
                    // Return the host to its original spot in the slide.
                    if let Some(ref host) = host_el
                        && let Some((parent, next)) = fullscreen_origin.take()
                        && parent.insert_before(host, next.as_ref()).is_err()
                    {
                        error!("Failed to restore terminal host into slide on collapse");
                    }
                    let size = *font_size_kbd.borrow();
                    last_dims = refit(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        window_el_kbd.as_ref(),
                        size,
                    )
                    .await;
                    let _ = canvas_kbd.focus();
                }
                KeyAction::Resize => {
                    // Skip the redundant repaint when a spurious observer callback
                    // resolves to the same grid we last sent the server.
                    let size = *font_size_kbd.borrow();
                    if compute_terminal_size(window_el_kbd.as_ref(), size) != last_dims {
                        last_dims = refit(
                            &vterm_kbd,
                            &canvas_kbd,
                            &ws_write_kbd,
                            window_el_kbd.as_ref(),
                            size,
                        )
                        .await;
                    }
                }
            }
        }
    });

    // Read terminal output from server
    let mut current_title = String::new();
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
            Ok(Message::Bytes(data)) => {
                vterm_rc.borrow_mut().process(&data);
                let vterm = vterm_rc.borrow();
                vterm.render_to_canvas(&canvas, *font_size.borrow());
                update_title(title_el.as_ref(), &mut current_title, vterm.title());
            }
            Ok(Message::Text(text)) => {
                vterm_rc.borrow_mut().process(text.as_bytes());
                let vterm = vterm_rc.borrow();
                vterm.render_to_canvas(&canvas, *font_size.borrow());
                update_title(title_el.as_ref(), &mut current_title, vterm.title());
            }
            Err(err) => {
                error!("Terminal WebSocket error:", err.to_string());
                break;
            }
        }
    }

    info!("Terminal session ended");
    let _ = ws_write.lock().await.close().await;
    // Drop both halves so the underlying socket is released and the server sees
    // the close.
    drop(ws_read);
}

fn setup_keyboard_handler(canvas: &HtmlCanvasElement, tx: mpsc::UnboundedSender<KeyAction>) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: KeyboardEvent| {
        let key = event.key();
        let meta = event.meta_key();

        // Cmd+/Cmd- for font size (don't send to terminal)
        if meta && (key == "=" || key == "+") {
            event.prevent_default();
            if tx.unbounded_send(KeyAction::FontIncrease).is_err() {
                return;
            }
            return;
        }
        if meta && key == "-" {
            event.prevent_default();
            if tx.unbounded_send(KeyAction::FontDecrease).is_err() {
                return;
            }
            return;
        }

        event.prevent_default();
        event.stop_propagation();

        let input = translate_key(&event);
        if !input.is_empty() {
            // Ignore send errors: session may have ended while handlers are still registered.
            let _ = tx.unbounded_send(KeyAction::Input(input));
        }
    });

    if canvas
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .is_err()
    {
        error!("Failed to register keyboard handler on canvas");
    }
    closure.forget();
}

fn translate_key(event: &KeyboardEvent) -> String {
    let key = event.key();
    let ctrl = event.ctrl_key();

    // Control key combinations (Ctrl only, not Cmd)
    if ctrl {
        return match key.as_str() {
            "c" => "\x03".to_owned(),
            "d" => "\x04".to_owned(),
            "z" => "\x1a".to_owned(),
            "l" => "\x0c".to_owned(),
            "a" => "\x01".to_owned(),
            "e" => "\x05".to_owned(),
            "u" => "\x15".to_owned(),
            "k" => "\x0b".to_owned(),
            "w" => "\x17".to_owned(),
            "r" => "\x12".to_owned(),
            _ => String::new(),
        };
    }

    // Special keys
    match key.as_str() {
        "Enter" => "\r".to_owned(),
        "Backspace" => "\x7f".to_owned(),
        "Tab" => "\t".to_owned(),
        "Escape" => "\x1b".to_owned(),
        "ArrowUp" => "\x1b[A".to_owned(),
        "ArrowDown" => "\x1b[B".to_owned(),
        "ArrowRight" => "\x1b[C".to_owned(),
        "ArrowLeft" => "\x1b[D".to_owned(),
        "Home" => "\x1b[H".to_owned(),
        "End" => "\x1b[F".to_owned(),
        "Delete" => "\x1b[3~".to_owned(),
        "PageUp" => "\x1b[5~".to_owned(),
        "PageDown" => "\x1b[6~".to_owned(),
        // Single printable character
        ch if ch.len() == 1 => ch.to_owned(),
        // Ignore modifier-only keys, etc.
        _ => String::new(),
    }
}

/// Titlebar height in pixels (CSS: .terminal-titlebar { height: 36px })
const TITLEBAR_HEIGHT: f64 = 36.0;
/// Body vertical padding: top 2px + bottom 3px (CSS: .terminal-body { padding: 2px 3px 3px })
const BODY_PADDING: f64 = 5.0;
/// Body horizontal padding: left 3px + right 3px (CSS: .terminal-body { padding: 2px 3px 3px })
const BODY_H_PADDING: f64 = 6.0;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn compute_terminal_size(window_el: Option<&HtmlElement>, font_size: f64) -> (u16, u16) {
    // Measure the same font metrics the renderer uses so the row/col count matches
    // the canvas cell size; fall back to the width/height heuristics when the font
    // cannot be measured yet.
    let (char_width, char_height) = vterm::cell_metrics_for(font_size).map_or(
        (
            font_size * vterm::FALLBACK_WIDTH_RATIO,
            font_size * vterm::FALLBACK_HEIGHT_RATIO,
        ),
        |metrics| (metrics.char_width, metrics.char_height),
    );

    let (avail_w, avail_h) = window_el
        .map(|el| (f64::from(el.client_width()), f64::from(el.client_height())))
        .filter(|(width, height)| *width > 0.0 && *height > 0.0)
        .unwrap_or((
            f64::from(DEFAULT_COLS) * char_width + BODY_H_PADDING,
            f64::from(DEFAULT_ROWS) * char_height + TITLEBAR_HEIGHT + BODY_PADDING,
        ));

    let body_width = avail_w - BODY_H_PADDING;
    let body_height = avail_h - TITLEBAR_HEIGHT - BODY_PADDING;

    let cols = (body_width / char_width).floor() as u16;
    let rows = (body_height / char_height).floor() as u16;

    (cols.max(MIN_COLS), rows.max(MIN_ROWS))
}

async fn resize_and_render(
    vterm: &Rc<RefCell<VirtualTerminal>>,
    canvas: &HtmlCanvasElement,
    ws_write: &Rc<futures::lock::Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
    cols: u16,
    rows: u16,
    font_size: f64,
) {
    {
        let mut vt = vterm.borrow_mut();
        vt.resize(cols, rows);
        vt.render_to_canvas(canvas, font_size);
    }
    let resize_msg = format!(r#"{{"type":"resize","cols":{cols},"rows":{rows}}}"#);
    if ws_write
        .lock()
        .await
        .send(Message::Bytes(resize_msg.into_bytes()))
        .await
        .is_err()
    {
        error!("Failed to send resize to server");
    }
}

/// Recomputes the grid for `font_size`, resizes and repaints the terminal, and
/// returns the new `(cols, rows)`. Shared by the font-resize and fullscreen arms.
async fn refit(
    vterm: &Rc<RefCell<VirtualTerminal>>,
    canvas: &HtmlCanvasElement,
    ws_write: &Rc<futures::lock::Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
    window_el: Option<&HtmlElement>,
    font_size: f64,
) -> (u16, u16) {
    let (cols, rows) = compute_terminal_size(window_el, font_size);
    resize_and_render(vterm, canvas, ws_write, cols, rows, font_size).await;
    (cols, rows)
}

fn update_title(title_el: Option<&HtmlElement>, current: &mut String, new_title: Option<&str>) {
    if let (Some(el), Some(title)) = (title_el, new_title.filter(|val| *val != current.as_str())) {
        el.set_text_content(Some(title));
        *current = title.to_owned();
    }
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

    url
}
