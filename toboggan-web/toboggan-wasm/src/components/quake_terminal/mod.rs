use std::cell::RefCell;
use std::rc::Rc;

use gloo::console::{debug, info};
use gloo::utils::document;
use toboggan_core::TerminalConfig;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{AddEventListenerOptions, Event, EventTarget, HtmlElement};

use crate::components::{TobogganTerminalElement, WasmElement};
use crate::{create_html_element, install_capture_keydown, is_editable_target};

const CSS: &str = include_str!("style.css");
const STYLE_MARKER_ATTR: &str = "data-toboggan-quake-style";
const TOGGLE_KEY: &str = "`";
const FALLBACK_CWD: &str = ".";

/// Drop-down "Quake-style" terminal overlay toggled by the backtick key.
///
/// The session's working directory is sourced from the active slide's resolved
/// `quake_terminal_cwd`. The loader applies the slide→talk fallback only, which
/// is why this module keeps its own [`FALLBACK_CWD`] for the remaining case.
///
/// When the resolved cwd changes between slides, the existing session is shut
/// down — its WebSocket closed, and with it the server-side PTY — and a new one
/// is opened in the new directory.
#[derive(Default)]
pub(crate) struct TobogganQuakeTerminalElement {
    state: Option<Rc<RefCell<QuakeState>>>,
}

struct QuakeState {
    overlay: HtmlElement,
    inner: TobogganTerminalElement,
    api_base_url: String,
    /// The cwd currently driving the running PTY session, or `None` when no
    /// session has been started yet.
    ///
    /// Distinct from [`Self::is_open`]: closing the overlay hides it but leaves
    /// the session running, so a reopen is instant. `is_open == true` therefore
    /// always implies `active_cwd.is_some()` — `toggle` starts a session before
    /// it sets the flag — but the converse does not hold.
    active_cwd: Option<String>,
    /// The cwd that should be used the next time the overlay is opened.
    pending_cwd: Option<String>,
    /// Whether the overlay is currently visible.
    is_open: bool,
}

impl TobogganQuakeTerminalElement {
    pub(crate) fn set_api_base_url(&mut self, url: &str) {
        if let Some(state) = &self.state {
            url.clone_into(&mut state.borrow_mut().api_base_url);
        }
    }

    /// Update the slide-derived cwd. If the overlay is currently open and the
    /// cwd changed, the session is restarted immediately. Otherwise the new cwd
    /// is staged for the next open.
    pub(crate) fn set_slide_cwd(&self, cwd: Option<String>) {
        let Some(state_rc) = &self.state else {
            return;
        };

        let needs_restart = {
            let mut state = state_rc.borrow_mut();
            let changed = state.is_open && state.active_cwd != cwd;
            state.pending_cwd = cwd;
            changed
        };

        if needs_restart {
            restart_session(state_rc);
        }
    }
}

impl WasmElement for TobogganQuakeTerminalElement {
    fn render(&mut self, _host: &HtmlElement) {
        inject_quake_style();

        // Render directly under <body> so the fixed-position overlay stacks above
        // the slide deck regardless of where this element was mounted.
        let body = document().body().unwrap_throw();

        let overlay = create_html_element("div");
        overlay.set_class_name("toboggan-quake-terminal");

        let inner_host = create_html_element("div");
        inner_host.set_class_name("toboggan-quake-inner");
        overlay.append_child(&inner_host).unwrap_throw();
        body.append_child(&overlay).unwrap_throw();

        // This host is created once and re-populated on every restart. Nothing
        // has to be done to protect it: maximizing lifts the window into the top
        // layer rather than moving it, so no terminal's host ever leaves the
        // place it was rendered into.
        let mut inner = TobogganTerminalElement::default();
        inner.render(&inner_host);

        let state = Rc::new(RefCell::new(QuakeState {
            overlay,
            inner,
            api_base_url: String::new(),
            active_cwd: None,
            pending_cwd: None,
            is_open: false,
        }));

        register_toggle_listener(Rc::clone(&state));
        register_click_outside_listener(Rc::clone(&state));
        self.state = Some(state);
    }
}

fn inject_quake_style() {
    let Some(head) = document().head() else {
        return;
    };
    let selector = format!("style[{STYLE_MARKER_ATTR}]");
    if matches!(head.query_selector(&selector), Ok(Some(_))) {
        return;
    }
    let style = create_html_element("style");
    style
        .set_attribute(STYLE_MARKER_ATTR, "true")
        .unwrap_throw();
    style.set_text_content(Some(CSS));
    head.append_child(&style).unwrap_throw();
}

fn register_toggle_listener(state: Rc<RefCell<QuakeState>>) {
    // Capture phase so we run before the inner terminal's own keydown handler
    // (which otherwise eats every key for the PTY).
    install_capture_keydown(move |event| {
        if event.key() != TOGGLE_KEY {
            return;
        }
        // Don't hijack backtick when the user is typing into a regular form
        // field (search box, contenteditable). The PTY canvas is a <canvas>,
        // not an editable element, so it's not matched here — backtick will
        // toggle even while focus is in the terminal, which is intentional.
        if is_editable_target(event) {
            return;
        }
        event.prevent_default();
        event.stop_propagation();
        toggle(&state);
    });
}

/// Closes the overlay when a click lands anywhere outside it.
///
/// The overlay only covers the top of the viewport, so the rest of the slide
/// stays clickable while it is down. Clicking there is the natural "I am done
/// with the terminal" gesture, and it is the only way out other than the toggle
/// key — the deck's own keys stay inert until one of the two happens.
///
/// Capture phase on the document, and `composedPath` rather than `target`,
/// because a click inside the terminal is retargeted to its shadow host and
/// would otherwise be indistinguishable from a click on the overlay's edge.
fn register_click_outside_listener(state: Rc<RefCell<QuakeState>>) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        let inside = {
            let quake = state.borrow();
            if !quake.is_open {
                return;
            }
            let overlay: &EventTarget = quake.overlay.as_ref();
            event
                .composed_path()
                .iter()
                .any(|node| node.dyn_ref::<EventTarget>() == Some(overlay))
        };
        if !inside {
            toggle(&state);
        }
    });

    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    document()
        .add_event_listener_with_callback_and_add_event_listener_options(
            "click",
            closure.as_ref().unchecked_ref(),
            &options,
        )
        .unwrap_throw();
    closure.forget();
}

fn toggle(state_rc: &Rc<RefCell<QuakeState>>) {
    let (will_open, needs_start) = {
        let state = state_rc.borrow();
        let will_open = !state.is_open;
        let needs_start =
            will_open && (state.active_cwd.is_none() || state.active_cwd != state.pending_cwd);
        (will_open, needs_start)
    };

    if needs_start {
        restart_session(state_rc);
    }

    let mut state = state_rc.borrow_mut();
    state.is_open = will_open;
    let class_list = state.overlay.class_list();
    if will_open {
        let _ = class_list.add_1("open");
        // While the overlay is down every key belongs to the shell, including
        // the ones the deck binds. Ownership is claimed through the inner
        // terminal rather than set here, so the overlay and a slide's terminal
        // take the keyboard by exactly the same route — the overlay only covers
        // the top of the viewport, and this is what keeps its keys its own even
        // when a click lands on the slide behind it.
        state.inner.capture_keyboard();
        info!("🎮 QuakeTerminal opened");
    } else {
        let _ = class_list.remove_1("open");
        // The session keeps running for an instant reopen, so releasing is also
        // what takes focus back off its hidden textarea.
        state.inner.release_keyboard();
        debug!("QuakeTerminal closed");
    }
}

fn restart_session(state_rc: &Rc<RefCell<QuakeState>>) {
    let mut state = state_rc.borrow_mut();
    // Reuse the existing inner element (its shadow root was created once at
    // render-time; re-rendering would call attachShadow again and throw).
    // stop_terminal clears the shadow container's children, and start_terminal
    // re-populates it with a fresh window/canvas.
    state.inner.stop_terminal();

    let cwd = state
        .pending_cwd
        .clone()
        .unwrap_or_else(|| FALLBACK_CWD.to_owned());
    let config = TerminalConfig::new(cwd.clone());
    let api_base = state.api_base_url.clone();

    state.active_cwd = Some(cwd);
    state.inner.start_terminal(&config, &api_base);

    // A restart tears the old session down, and tearing down releases its claim
    // on the keyboard. Reclaiming matters when the restart came from a slide
    // change rather than a toggle: the overlay is still down, so its keys are
    // still its own.
    if state.is_open {
        state.inner.capture_keyboard();
    }
}
