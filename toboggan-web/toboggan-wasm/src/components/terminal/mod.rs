mod vterm;

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use gloo::console::{error, info};
use gloo::net::websocket::Message;
use gloo::net::websocket::futures::WebSocket;
use toboggan_core::{TerminalConfig, Theme};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, HtmlCanvasElement, HtmlElement, KeyboardEvent};

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

#[derive(Debug, Default)]
pub(crate) struct TobogganTerminalElement {
    container: Option<Element>,
}

impl TobogganTerminalElement {
    pub(crate) fn start_terminal(&self, config: &TerminalConfig, api_base_url: &str) {
        let Some(container) = &self.container else {
            error!("start_terminal called before render");
            return;
        };

        let document = gloo::utils::document();
        let is_light = config.theme == Theme::Light;

        let window_class = if is_light {
            "terminal-window terminal-light"
        } else {
            "terminal-window terminal-dark"
        };
        let window_el = create_element_with_class(&document, "div", window_class);

        let titlebar = create_element_with_class(&document, "div", "terminal-titlebar");

        let buttons = create_element_with_class(&document, "div", "terminal-buttons");
        let btn_close =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-close");
        let btn_minimize =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-minimize");
        let btn_maximize =
            create_element_with_class(&document, "div", "terminal-btn terminal-btn-maximize");
        let _ = buttons.append_child(&btn_close);
        let _ = buttons.append_child(&btn_minimize);
        let _ = buttons.append_child(&btn_maximize);
        let _ = titlebar.append_child(&buttons);

        let title_text = create_element_with_class(&document, "span", "terminal-title");
        let title = config
            .cmd
            .as_deref()
            .or_else(|| config.cwd.rsplit('/').find(|segment| !segment.is_empty()))
            .unwrap_or(&config.cwd);
        title_text.set_text_content(Some(title));
        let _ = titlebar.append_child(&title_text);
        let _ = window_el.append_child(&titlebar);

        let body = create_element_with_class(&document, "div", "terminal-body");

        let Ok(canvas) = document.create_element("canvas") else {
            error!("Failed to create canvas element");
            return;
        };
        let Ok(canvas) = canvas.dyn_into::<HtmlCanvasElement>() else {
            error!("Failed to cast to HtmlCanvasElement");
            return;
        };
        canvas.set_class_name("terminal-canvas");
        canvas.set_attribute("tabindex", "0").unwrap_throw();

        let _ = body.append_child(&canvas);
        let _ = window_el.append_child(&body);
        let _ = container.append_child(&window_el);

        canvas.focus().unwrap_throw();

        let theme = config.theme;
        let title_el = title_text.dyn_into::<HtmlElement>().ok();
        let window_html = window_el.dyn_into::<HtmlElement>().ok();
        let initial_rows = compute_terminal_size(window_html.as_ref(), DEFAULT_FONT_SIZE).1;
        let ws_url = build_terminal_ws_url(api_base_url, config, initial_rows);

        // Set up action channel (shared between keyboard handler and button clicks)
        let (tx_key, rx_key) = mpsc::unbounded::<KeyAction>();
        setup_keyboard_handler(&canvas, tx_key.clone());
        setup_button_click(&btn_maximize, tx_key.clone(), KeyAction::Expand);
        setup_button_click(&btn_minimize, tx_key, KeyAction::Restore);

        info!("Starting terminal session:", &ws_url);

        spawn_local(async move {
            run_terminal_session(
                canvas,
                &ws_url,
                theme,
                title_el,
                window_html,
                initial_rows,
                rx_key,
            )
            .await;
        });
    }

    pub(crate) fn stop_terminal(&self) {
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

        self.container = Some(container);
    }
}

fn create_element_with_class(document: &web_sys::Document, tag: &str, class: &str) -> Element {
    let Ok(el) = document.create_element(tag) else {
        return document
            .create_element("div")
            .unwrap_or_else(|_| unreachable!("could not create div element"));
    };
    el.set_class_name(class);
    el
}

/// Message from keyboard/button handler to terminal session
#[derive(Clone)]
enum KeyAction {
    Input(String),
    FontIncrease,
    FontDecrease,
    Expand,
    Restore,
}

fn setup_button_click(btn: &Element, tx: mpsc::UnboundedSender<KeyAction>, action: KeyAction) {
    let closure = Closure::<dyn FnMut()>::new(move || {
        let _ = tx.unbounded_send(action.clone());
    });
    let _ = btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
    closure.forget();
}

#[allow(clippy::await_holding_refcell_ref, clippy::too_many_lines)] // Safe: single-threaded WASM
async fn run_terminal_session(
    canvas: HtmlCanvasElement,
    ws_url: &str,
    theme: Theme,
    title_el: Option<HtmlElement>,
    window_el: Option<HtmlElement>,
    initial_rows: u16,
    rx_key: mpsc::UnboundedReceiver<KeyAction>,
) {
    let ws = match WebSocket::open(ws_url) {
        Ok(ws) => ws,
        Err(err) => {
            error!("Failed to connect to terminal:", err.to_string());
            return;
        }
    };

    let (ws_write, mut ws_read) = ws.split();
    let font_size = Rc::new(RefCell::new(DEFAULT_FONT_SIZE));

    let vterm = VirtualTerminal::new(DEFAULT_COLS, initial_rows, theme);

    vterm.render_to_canvas(&canvas, *font_size.borrow());

    // Forward actions (keyboard input, font resize, expand/restore) to WebSocket
    let ws_write = Rc::new(RefCell::new(ws_write));
    let ws_write_kbd = Rc::clone(&ws_write);
    let font_size_kbd = Rc::clone(&font_size);
    let canvas_kbd = canvas.clone();
    let vterm_rc = Rc::new(RefCell::new(vterm));
    let vterm_kbd = Rc::clone(&vterm_rc);
    let window_el_kbd = window_el.clone();

    spawn_local(async move {
        let mut rx_key = rx_key;
        while let Some(action) = rx_key.next().await {
            match action {
                KeyAction::Input(input) => {
                    let send_result = ws_write_kbd.borrow_mut().send(Message::Text(input)).await;
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
                    let (new_cols, new_rows) =
                        compute_terminal_size(window_el_kbd.as_ref(), new_size);
                    resize_and_render(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        new_cols,
                        new_rows,
                        new_size,
                    )
                    .await;
                }
                KeyAction::Expand => {
                    if let Some(ref win) = window_el_kbd {
                        let _ = win.class_list().add_1("terminal-fullscreen");
                    }
                    let size = *font_size_kbd.borrow();
                    let (new_cols, new_rows) = compute_terminal_size(window_el_kbd.as_ref(), size);
                    resize_and_render(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        new_cols,
                        new_rows,
                        size,
                    )
                    .await;
                    let _ = canvas_kbd.focus();
                }
                KeyAction::Restore => {
                    if let Some(ref win) = window_el_kbd {
                        let _ = win.class_list().remove_1("terminal-fullscreen");
                    }
                    let size = *font_size_kbd.borrow();
                    let (new_cols, new_rows) = compute_terminal_size(window_el_kbd.as_ref(), size);
                    resize_and_render(
                        &vterm_kbd,
                        &canvas_kbd,
                        &ws_write_kbd,
                        new_cols,
                        new_rows,
                        size,
                    )
                    .await;
                    let _ = canvas_kbd.focus();
                }
            }
        }
    });

    // Read terminal output from server
    let mut current_title = String::new();
    while let Some(msg) = ws_read.next().await {
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
    let _ = ws_write.borrow_mut().close().await;
}

fn setup_keyboard_handler(canvas: &HtmlCanvasElement, tx: mpsc::UnboundedSender<KeyAction>) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: KeyboardEvent| {
        let key = event.key();
        let meta = event.meta_key();

        // Cmd+/Cmd- for font size (don't send to terminal)
        if meta && (key == "=" || key == "+") {
            event.prevent_default();
            let _ = tx.unbounded_send(KeyAction::FontIncrease);
            return;
        }
        if meta && key == "-" {
            event.prevent_default();
            let _ = tx.unbounded_send(KeyAction::FontDecrease);
            return;
        }

        event.prevent_default();
        event.stop_propagation();

        let input = translate_key(&event);
        if !input.is_empty() {
            let _ = tx.unbounded_send(KeyAction::Input(input));
        }
    });

    canvas
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap_throw();
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
    let char_height = (font_size * 1.3).ceil();
    // Character width approximation (0.6 ratio of font size)
    let char_width = (font_size * 0.6).ceil();

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

    (cols.max(20), rows.max(4))
}

#[allow(clippy::await_holding_refcell_ref)] // Safe: single-threaded WASM
async fn resize_and_render(
    vterm: &Rc<RefCell<VirtualTerminal>>,
    canvas: &HtmlCanvasElement,
    ws_write: &Rc<
        RefCell<futures::stream::SplitSink<WebSocket, Message>>,
    >,
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
        .borrow_mut()
        .send(Message::Bytes(resize_msg.into_bytes()))
        .await
        .is_err()
    {
        error!("Failed to send resize to server");
    }
}

fn update_title(title_el: Option<&HtmlElement>, current: &mut String, new_title: Option<&str>) {
    if let (Some(el), Some(title)) = (title_el, new_title.filter(|val| *val != current.as_str())) {
        el.set_text_content(Some(title));
        *current = title.to_owned();
    }
}

fn build_terminal_ws_url(api_base_url: &str, config: &TerminalConfig, rows: u16) -> String {
    let ws_base = api_base_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    let encoded_cwd = String::from(js_sys::encode_uri_component(&config.cwd));
    let mut url =
        format!("{ws_base}/api/terminal?cwd={encoded_cwd}&cols={DEFAULT_COLS}&rows={rows}");

    if let Some(cmd) = &config.cmd {
        url.push_str("&cmd=");
        url.push_str(&String::from(js_sys::encode_uri_component(cmd)));
    }

    url
}
