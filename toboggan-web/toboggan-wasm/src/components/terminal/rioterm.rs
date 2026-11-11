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

use std::borrow::Cow;

use serde::Serialize;
use toboggan_core::Theme;
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
/// used, so replacing the engine does not restyle every existing deck. A deck
/// recolours any slot through the custom property [`Self::slots_mut`] names.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RioTheme {
    pub(super) foreground: Cow<'static, str>,
    pub(super) background: Cow<'static, str>,
    pub(super) cursor: Cow<'static, str>,
    pub(super) selection_foreground: Cow<'static, str>,
    pub(super) selection_background: Cow<'static, str>,
    pub(super) black: Cow<'static, str>,
    pub(super) red: Cow<'static, str>,
    pub(super) green: Cow<'static, str>,
    pub(super) yellow: Cow<'static, str>,
    pub(super) blue: Cow<'static, str>,
    pub(super) magenta: Cow<'static, str>,
    pub(super) cyan: Cow<'static, str>,
    pub(super) white: Cow<'static, str>,
    pub(super) bright_black: Cow<'static, str>,
    pub(super) bright_red: Cow<'static, str>,
    pub(super) bright_green: Cow<'static, str>,
    pub(super) bright_yellow: Cow<'static, str>,
    pub(super) bright_blue: Cow<'static, str>,
    pub(super) bright_magenta: Cow<'static, str>,
    pub(super) bright_cyan: Cow<'static, str>,
    pub(super) bright_white: Cow<'static, str>,
}

impl RioTheme {
    /// The built-in palette a deck in `theme` starts from, before its own
    /// `--terminal-<slot>` properties are applied.
    pub(super) fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self::LIGHT,
            Theme::Dark => Self::DARK,
        }
    }

    /// Every colour slot, paired with the custom property that overrides it.
    ///
    /// The property is the field name in kebab-case behind `--terminal-`, so a
    /// deck reads the same vocabulary as rioterm's own `Theme`. Written out
    /// rather than derived: the list is the public contract, and a renamed field
    /// must not rename a property some deck already declares.
    pub(super) fn slots_mut(&mut self) -> [(&'static str, &mut Cow<'static, str>); 21] {
        [
            ("--terminal-foreground", &mut self.foreground),
            ("--terminal-background", &mut self.background),
            ("--terminal-cursor", &mut self.cursor),
            (
                "--terminal-selection-foreground",
                &mut self.selection_foreground,
            ),
            (
                "--terminal-selection-background",
                &mut self.selection_background,
            ),
            ("--terminal-black", &mut self.black),
            ("--terminal-red", &mut self.red),
            ("--terminal-green", &mut self.green),
            ("--terminal-yellow", &mut self.yellow),
            ("--terminal-blue", &mut self.blue),
            ("--terminal-magenta", &mut self.magenta),
            ("--terminal-cyan", &mut self.cyan),
            ("--terminal-white", &mut self.white),
            ("--terminal-bright-black", &mut self.bright_black),
            ("--terminal-bright-red", &mut self.bright_red),
            ("--terminal-bright-green", &mut self.bright_green),
            ("--terminal-bright-yellow", &mut self.bright_yellow),
            ("--terminal-bright-blue", &mut self.bright_blue),
            ("--terminal-bright-magenta", &mut self.bright_magenta),
            ("--terminal-bright-cyan", &mut self.bright_cyan),
            ("--terminal-bright-white", &mut self.bright_white),
        ]
    }

    /// Catppuccin Mocha, matching the old emulator's `DARK_COLORS`.
    pub(super) const DARK: Self = Self {
        foreground: Cow::Borrowed("#cdd6f4"),
        background: Cow::Borrowed("#1e1e2e"),
        cursor: Cow::Borrowed("#f5e0dc"),
        selection_foreground: Cow::Borrowed("#cdd6f4"),
        selection_background: Cow::Borrowed("#585b70"),
        black: Cow::Borrowed("#45475a"),
        red: Cow::Borrowed("#f38ba8"),
        green: Cow::Borrowed("#a6e3a1"),
        yellow: Cow::Borrowed("#f9e2af"),
        blue: Cow::Borrowed("#89b4fa"),
        magenta: Cow::Borrowed("#cba6f7"),
        cyan: Cow::Borrowed("#94e2d5"),
        white: Cow::Borrowed("#bac2de"),
        bright_black: Cow::Borrowed("#585b70"),
        bright_red: Cow::Borrowed("#f38ba8"),
        bright_green: Cow::Borrowed("#a6e3a1"),
        bright_yellow: Cow::Borrowed("#f9e2af"),
        bright_blue: Cow::Borrowed("#89b4fa"),
        bright_magenta: Cow::Borrowed("#cba6f7"),
        bright_cyan: Cow::Borrowed("#94e2d5"),
        bright_white: Cow::Borrowed("#cdd6f4"),
    };

    /// Catppuccin Latte, matching the old emulator's `LIGHT_COLORS`.
    pub(super) const LIGHT: Self = Self {
        foreground: Cow::Borrowed("#4c4f69"),
        background: Cow::Borrowed("#eff1f5"),
        cursor: Cow::Borrowed("#dc8a78"),
        selection_foreground: Cow::Borrowed("#4c4f69"),
        selection_background: Cow::Borrowed("#bcc0cc"),
        black: Cow::Borrowed("#acb0be"),
        red: Cow::Borrowed("#d20f39"),
        green: Cow::Borrowed("#40a02b"),
        yellow: Cow::Borrowed("#df8e1d"),
        blue: Cow::Borrowed("#1e66f5"),
        magenta: Cow::Borrowed("#8839ef"),
        cyan: Cow::Borrowed("#179299"),
        white: Cow::Borrowed("#5c5f77"),
        bright_black: Cow::Borrowed("#bcc0cc"),
        bright_red: Cow::Borrowed("#d20f39"),
        bright_green: Cow::Borrowed("#40a02b"),
        bright_yellow: Cow::Borrowed("#df8e1d"),
        bright_blue: Cow::Borrowed("#1e66f5"),
        bright_magenta: Cow::Borrowed("#8839ef"),
        bright_cyan: Cow::Borrowed("#179299"),
        bright_white: Cow::Borrowed("#4c4f69"),
    };
}
