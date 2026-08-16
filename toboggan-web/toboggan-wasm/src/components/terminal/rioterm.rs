//! Bindings to the `rioterm` npm package: Rio's terminal core (`librio`)
//! compiled to WebAssembly, behind an xterm.js-shaped API.
//!
//! `wasm-bindgen` turns the `module = "rioterm"` attribute into a bare
//! `import { … } from 'rioterm'` at the top of the generated glue, which vite
//! resolves from `toboggan-web/node_modules` — the glue is a source file that
//! `main.ts` imports, so it goes through the bundler like any other module.
//! Only the symbols actually used below are imported.
//!
//! rioterm loads its own wasm lazily (`wasmReady ??= init(…)`, reached only
//! from `open()` and `Terminal.create()`), so a deck with no terminal slide
//! never fetches it.
//!
//! The surface here is deliberately small. `open()` already wires the keyboard
//! (through its own hidden textarea), mouse selection, OSC 8 link hover and
//! activation, wheel scrollback, and clipboard copy/paste — none of which needs
//! binding. This module is bindings only; the session logic lives in the parent
//! module.

use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::HtmlElement;

#[wasm_bindgen(module = "rioterm")]
extern "C" {
    /// Mounts a terminal into `parent` and resolves to a `RioTermHandle`.
    ///
    /// Rejects rather than throwing synchronously, hence `catch`: a failed wasm
    /// fetch (offline, blocked by CSP) must degrade to a logged error, not an
    /// unwind — the crate builds with `panic = "abort"`.
    #[wasm_bindgen(js_name = "open", catch)]
    pub(super) async fn open(parent: &HtmlElement, options: &JsValue) -> Result<JsValue, JsValue>;

    /// What `open()` resolves to. A plain object literal, not a class — reach
    /// it with `unchecked_into`, never `dyn_into`, since there is no
    /// constructor to test against.
    pub(super) type RioTermHandle;

    #[wasm_bindgen(method, getter)]
    pub(super) fn terminal(this: &RioTermHandle) -> Terminal;

    #[wasm_bindgen(method, getter)]
    pub(super) fn renderer(this: &RioTermHandle) -> CanvasRenderer;

    /// Focuses the hidden textarea that receives keystrokes.
    #[wasm_bindgen(method, js_name = "focus")]
    pub(super) fn focus(this: &RioTermHandle);

    /// Tears down terminal, renderer, every listener, and the container element
    /// `open()` inserted. Leaves `parent` as it found it.
    #[wasm_bindgen(method, js_name = "dispose")]
    pub(super) fn dispose(this: &RioTermHandle);

    /// Turns predictive local echo on or off at runtime. A no-op unless the
    /// terminal was opened with `predictive_echo`, since that is when the
    /// engine is created.
    #[wasm_bindgen(method, js_name = "setPredictiveEcho")]
    pub(super) fn set_predictive_echo(this: &RioTermHandle, enabled: bool);

    /// The VT state machine and grid.
    pub(super) type Terminal;

    /// Backend output to display.
    #[wasm_bindgen(method)]
    pub(super) fn write(this: &Terminal, data: &[u8]);

    /// Bytes the terminal wants delivered to the backend: keystrokes, mouse
    /// reports, and the replies to Device Attributes / cursor-position queries
    /// that the server used to answer by sniffing the PTY stream.
    #[wasm_bindgen(method, js_name = "onData")]
    pub(super) fn on_data(this: &Terminal, callback: &JsValue) -> JsValue;

    /// OSC 0/2 title changes, for the window titlebar.
    #[wasm_bindgen(method, js_name = "onTitleChange")]
    pub(super) fn on_title_change(this: &Terminal, callback: &JsValue) -> JsValue;

    /// The live option bag. `resize()` writes the settled grid back into it, so
    /// this is where the current size is read from — `Terminal` itself exposes
    /// no `cols`/`rows` accessor.
    #[wasm_bindgen(method, getter)]
    pub(super) fn options(this: &Terminal) -> TerminalOptions;

    /// The whole buffer as a VT byte stream that reproduces content, styling
    /// and links when written into a fresh terminal. Used to carry the screen
    /// across a font-size change, which has to rebuild the terminal.
    #[wasm_bindgen(method)]
    pub(super) fn serialize(this: &Terminal) -> String;

    pub(super) type TerminalOptions;

    #[wasm_bindgen(method, getter)]
    pub(super) fn cols(this: &TerminalOptions) -> u16;

    #[wasm_bindgen(method, getter)]
    pub(super) fn rows(this: &TerminalOptions) -> u16;

    /// Canvas renderer over a `Terminal`.
    pub(super) type CanvasRenderer;

    /// Resizes the grid to fill `width` x `height` CSS pixels, deriving
    /// `cols`/`rows` from its own cell metrics. A no-op when the result matches
    /// the current grid.
    #[wasm_bindgen(method)]
    pub(super) fn fit(this: &CanvasRenderer, width: f64, height: f64);

    /// The mount point, so the deck's own canvas styling can be applied to it.
    #[wasm_bindgen(method, getter)]
    pub(super) fn element(this: &CanvasRenderer) -> HtmlElement;
}

/// Options for [`open`].
///
/// Serialized to a plain JS object with `serde_wasm_bindgen`; the keys are
/// camelCase on the JS side and matched exactly, so the rename is load-bearing
/// rather than cosmetic.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OpenOptions {
    /// Always `"canvas"`: the DOM renderer paints a node per row, which a
    /// projector-sized terminal cannot afford.
    pub(super) renderer: &'static str,
    /// `false` — rioterm's own fit would resize the grid without telling us,
    /// and the size has to reach the server's PTY, so the parent module drives
    /// fitting instead.
    pub(super) fit: bool,
    pub(super) auto_focus: bool,
    /// Predictive local echo: paint typed characters immediately and reconcile
    /// against the server's echo, so the WebSocket round-trip stops being felt.
    pub(super) predictive_echo: bool,
    pub(super) font_family: String,
    pub(super) font_size: f64,
    pub(super) theme: RioTheme,
}

/// rioterm's 16-colour palette plus the default and selection colours.
///
/// Carries the same Catppuccin Latte / Mocha values the hand-written emulator
/// used, so replacing the engine does not restyle every existing deck.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RioTheme {
    pub(super) foreground: &'static str,
    pub(super) background: &'static str,
    pub(super) cursor: &'static str,
    pub(super) selection_foreground: &'static str,
    pub(super) selection_background: &'static str,
    pub(super) black: &'static str,
    pub(super) red: &'static str,
    pub(super) green: &'static str,
    pub(super) yellow: &'static str,
    pub(super) blue: &'static str,
    pub(super) magenta: &'static str,
    pub(super) cyan: &'static str,
    pub(super) white: &'static str,
    pub(super) bright_black: &'static str,
    pub(super) bright_red: &'static str,
    pub(super) bright_green: &'static str,
    pub(super) bright_yellow: &'static str,
    pub(super) bright_blue: &'static str,
    pub(super) bright_magenta: &'static str,
    pub(super) bright_cyan: &'static str,
    pub(super) bright_white: &'static str,
}

impl RioTheme {
    /// Catppuccin Mocha, matching the old emulator's `DARK_COLORS`.
    pub(super) const DARK: Self = Self {
        foreground: "#cdd6f4",
        background: "#1e1e2e",
        cursor: "#f5e0dc",
        selection_foreground: "#cdd6f4",
        selection_background: "#585b70",
        black: "#45475a",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        blue: "#89b4fa",
        magenta: "#cba6f7",
        cyan: "#94e2d5",
        white: "#bac2de",
        bright_black: "#585b70",
        bright_red: "#f38ba8",
        bright_green: "#a6e3a1",
        bright_yellow: "#f9e2af",
        bright_blue: "#89b4fa",
        bright_magenta: "#cba6f7",
        bright_cyan: "#94e2d5",
        bright_white: "#cdd6f4",
    };

    /// Catppuccin Latte, matching the old emulator's `LIGHT_COLORS`.
    pub(super) const LIGHT: Self = Self {
        foreground: "#4c4f69",
        background: "#eff1f5",
        cursor: "#dc8a78",
        selection_foreground: "#4c4f69",
        selection_background: "#bcc0cc",
        black: "#acb0be",
        red: "#d20f39",
        green: "#40a02b",
        yellow: "#df8e1d",
        blue: "#1e66f5",
        magenta: "#8839ef",
        cyan: "#179299",
        white: "#5c5f77",
        bright_black: "#bcc0cc",
        bright_red: "#d20f39",
        bright_green: "#40a02b",
        bright_yellow: "#df8e1d",
        bright_blue: "#1e66f5",
        bright_magenta: "#8839ef",
        bright_cyan: "#179299",
        bright_white: "#4c4f69",
    };
}
