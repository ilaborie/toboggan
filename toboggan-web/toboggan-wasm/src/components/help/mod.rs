use std::fmt::Write as _;

use gloo::console::debug;
use gloo::utils::{document, window};
use toboggan_core::Command;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{AddEventListenerOptions, HtmlDialogElement, HtmlElement, KeyboardEvent};

use crate::components::WasmElement;
use crate::create_html_element;
use crate::services::KeyboardMapping;

const CSS: &str = include_str!("style.css");
const STYLE_MARKER_ATTR: &str = "data-toboggan-help-style";
const TOGGLE_KEY: &str = "F1";

/// Modal help dialog listing the active keyboard shortcuts.
///
/// Toggled with `F1` (capture-phase window listener so the inner terminal
/// canvas doesn't swallow the key). Closed with `Esc` or backdrop click via
/// the native `<dialog>` behavior.
#[derive(Default)]
pub(crate) struct TobogganHelpElement {
    mapping: KeyboardMapping,
    dialog: Option<HtmlDialogElement>,
}

impl TobogganHelpElement {
    pub(crate) fn set_mapping(&mut self, mapping: KeyboardMapping) {
        self.mapping = mapping;
    }
}

impl WasmElement for TobogganHelpElement {
    fn render(&mut self, _host: &HtmlElement) {
        inject_help_style();

        // Mount under <body> so the dialog's top-layer rendering stacks above
        // every other component regardless of where this element was hosted.
        let body = document().body().unwrap_throw();
        let dialog = create_html_element("dialog")
            .dyn_into::<HtmlDialogElement>()
            .unwrap_throw();
        dialog.set_class_name("toboggan-help");
        dialog.set_inner_html(&build_help_html(&self.mapping));
        body.append_child(&dialog).unwrap_throw();

        wire_close_button(&dialog);
        register_toggle_listener(dialog.clone());
        self.dialog = Some(dialog);
    }
}

fn inject_help_style() {
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

fn wire_close_button(dialog: &HtmlDialogElement) {
    let Ok(Some(btn)) = dialog.query_selector("button.close") else {
        return;
    };
    let btn = btn.dyn_into::<HtmlElement>().unwrap_throw();
    let dialog_clone = dialog.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
        dialog_clone.close();
    });
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap_throw();
    closure.forget();
}

fn register_toggle_listener(dialog: HtmlDialogElement) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: KeyboardEvent| {
        if event.key() != TOGGLE_KEY {
            return;
        }
        // F1 is a function key — never legitimate input — so unlike the quake
        // terminal's backtick we don't need to skip when an editable element
        // has focus.
        event.prevent_default();
        event.stop_propagation();
        if dialog.open() {
            dialog.close();
            debug!("📕 Help dialog closed");
        } else if dialog.show_modal().is_ok() {
            debug!("📖 Help dialog opened");
        }
    });
    let opts = AddEventListenerOptions::new();
    opts.set_capture(true);
    window()
        .add_event_listener_with_callback_and_add_event_listener_options(
            "keydown",
            closure.as_ref().unchecked_ref(),
            &opts,
        )
        .unwrap_throw();
    closure.forget();
}

fn build_help_html(mapping: &KeyboardMapping) -> String {
    let nav_rows = build_navigation_rows(mapping);

    let mut out = String::new();
    out.push_str(
        r#"<header><h2>Keyboard shortcuts</h2><button class="close" type="button" aria-label="Close">&times;</button></header>"#,
    );

    if !nav_rows.is_empty() {
        out.push_str("<h3>Navigation</h3><dl>");
        out.push_str(&nav_rows);
        out.push_str("</dl>");
    }

    out.push_str("<h3>Tools</h3><dl>");
    out.push_str("<dt><kbd>`</kbd></dt><dd>Toggle terminal overlay</dd>");
    out.push_str("<dt><kbd>F1</kbd></dt><dd>Toggle this help dialog</dd>");
    out.push_str("<dt><kbd>Esc</kbd></dt><dd>Close help dialog</dd>");
    out.push_str("</dl>");

    out
}

/// Group server-mapped keys by their command label and render one `<dt>/<dd>`
/// pair per command. Pure-letter aliases (`b`/`B`) collapse to the uppercase
/// form so the list reads as case-insensitive shortcuts.
fn build_navigation_rows(mapping: &KeyboardMapping) -> String {
    let mut groups: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for (key, cmd) in mapping.entries() {
        let label = command_label(cmd);
        if label.is_empty() {
            continue;
        }
        match groups.iter_mut().find(|(existing, _)| *existing == label) {
            Some((_, keys)) => keys.push(key),
            None => groups.push((label, vec![key])),
        }
    }
    groups.sort_by_key(|(label, _)| *label);

    let mut out = String::new();
    for (label, mut keys) in groups {
        keys.sort_unstable();
        keys.dedup_by(|first, second| first.eq_ignore_ascii_case(second));
        let kbds = keys
            .iter()
            .map(|key| format!("<kbd>{}</kbd>", display_key(key)))
            .collect::<Vec<_>>()
            .join(" / ");
        let _ = write!(out, "<dt>{kbds}</dt><dd>{label}</dd>");
    }
    out
}

fn display_key(key: &str) -> String {
    match key {
        "ArrowLeft" => "←".to_owned(),
        "ArrowUp" => "↑".to_owned(),
        "ArrowRight" => "→".to_owned(),
        "ArrowDown" => "↓".to_owned(),
        " " => "Space".to_owned(),
        other => other.to_ascii_uppercase(),
    }
}

fn command_label(cmd: &Command) -> &'static str {
    match cmd {
        Command::First => "First slide",
        Command::Last => "Last slide",
        Command::NextSlide => "Next slide",
        Command::PreviousSlide => "Previous slide",
        Command::NextStep => "Next step",
        Command::PreviousStep => "Previous step",
        Command::Blink => "Blink screen",
        Command::Ping
        | Command::Register { .. }
        | Command::Unregister { .. }
        | Command::GoTo { .. } => "",
    }
}
