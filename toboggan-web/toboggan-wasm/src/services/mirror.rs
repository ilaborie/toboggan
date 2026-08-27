//! The contract between the presenter view and the deck rendered inside its
//! panes.
//!
//! A pane is an `<iframe>` of the deck itself, so what it shows is the real
//! thing — the same stylesheet, the same footer, the same `_head.html`, laid out
//! against a viewport of its own. It reached that state after the alternative
//! had been tried and failed: rendering the slide component straight into the
//! presenter's shadow tree put the deck's arbitrary author CSS in the same
//! document as the speaker's chrome, where a deck that restyles `html` or
//! `body` restyled the presenter view along with it. An iframe is the only real
//! CSS boundary the platform offers.
//!
//! The frames are posted rather than fetched: the presenter already holds
//! everything a pane needs — it has a socket, and it fetched the slide to show
//! the notes. Giving each pane a socket of its own would put three clients in
//! `/api/clients` and fire a connect toast at the room for one person
//! presenting.

use gloo::console::{debug, error};
use gloo::utils::window;
use serde::{Deserialize, Serialize};
use toboggan_core::Slide;
use wasm_bindgen::JsValue;
use web_sys::MessageEvent;

/// Which pane a mirror is. Told to it in its URL, and echoed back on
/// [`MirrorMessage::Ready`] so the presenter knows which one has come up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MirrorPane {
    /// The slide the room is looking at, at the reveal it is looking at.
    Current,
    /// The slide after it, with every reveal shown at once: the point of the
    /// pane is to see what is coming, not to re-enact its build.
    Next,
}

impl MirrorPane {
    /// The value this pane is named by in `?mirror=`.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Next => "next",
        }
    }

    /// Reads a pane back out of `?mirror=`, or `None` for anything else —
    /// including a page that is simply not a mirror.
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "current" => Some(Self::Current),
            "next" => Some(Self::Next),
            _ => None,
        }
    }
}

/// Everything a mirror needs to paint the deck the way `/run` paints it.
///
/// Whole every time rather than a diff. A mirror is reloaded whenever the
/// browser feels like it — the back/forward cache, an iframe moved in the DOM,
/// the dev server — and a diff against a state the receiver may no longer hold
/// is a bug that only appears in front of a room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MirrorFrame {
    /// The deck's `_head.html`. Injected into the *mirror's* head, which is the
    /// whole point of the rebuild: injected into the presenter's head instead,
    /// a deck that styles `main` — as the packaged guide does — painted the
    /// speaker's chrome with the projector's backdrop, because `<main>` is the
    /// presenter shell's shadow host and an outer-document rule outranks
    /// `:host` whatever its specificity.
    pub head: Option<String>,
    /// Pre-rendered footer HTML, exactly as `TobogganFooterElement` takes it.
    pub footer: Option<String>,
    /// The deck's language tag. Carried because it decides hyphenation, and a
    /// pane that hyphenates differently from the projector breaks its lines
    /// somewhere else — the one thing a preview must not do.
    pub lang: Option<String>,
    /// `None` clears the pane — before the talk starts, and past the last slide.
    pub slide: Option<Slide>,
    /// Which reveal to stop at, or `None` for all of them, which is what the
    /// next-slide pane wants: the point of that pane is to see what is coming,
    /// not to re-enact its build.
    ///
    /// `None` rather than the `usize::MAX` the slide component takes for
    /// "everything". That value's JSON depends on the pointer width `usize`
    /// happens to have, and a sentinel in a wire format is a number that means
    /// something other than a number.
    pub step: Option<usize>,
    /// `init` / `running` / `done`, for `state.css`.
    pub state_class: String,
    /// 1-based slide number, for `--current-slide`.
    pub slide_number: usize,
    /// Deck length, for `--total-slides`.
    pub total_slides: usize,
}

/// What crosses the frame boundary, in both directions.
///
/// One tagged enum rather than a type per direction, so the two sides cannot
/// drift — and internally tagged like every other wire type in this workspace,
/// for the same reason plus one: a page is posted at by extensions, dev tools
/// and its own bundler, so "is this ours" has to be a cheap question with an
/// unambiguous answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mirror")]
pub(crate) enum MirrorMessage {
    /// Mirror → presenter, once it can accept frames.
    ///
    /// The presenter has state before its iframes finish loading: it came over a
    /// socket that was open before the `<iframe>` existed. Without this the
    /// first frame is posted into a document with no listener yet, and the pane
    /// stays blank until the speaker presses a key.
    Ready { pane: MirrorPane },
    /// Presenter → mirror.
    ///
    /// Boxed because it dwarfs `Ready`, and clippy is right that an enum the
    /// size of its largest variant is a poor trade when the small one is the one
    /// sent on every load.
    Frame(Box<MirrorFrame>),
}

/// This page's origin, or `None` if the browser will not say.
///
/// Every send and every receive is checked against it, so failing to read it
/// makes a page deaf and mute rather than indiscriminate — which is the right
/// way round for something that cannot be read.
pub(crate) fn page_origin() -> Option<String> {
    match window().location().origin() {
        Ok(origin) => Some(origin),
        Err(err) => {
            error!("Could not read this page's origin:", err);
            None
        }
    }
}

/// Encodes a message for `postMessage`.
///
/// JSON in a string rather than a cloned object graph. Both ends parse it with
/// the same `serde_json`, so there is no question of how a given browser chose
/// to structured-clone a map or how wide a `usize` was on the way through, and
/// a string is the one payload every `postMessage` implementation carries
/// identically.
pub(crate) fn encode(message: &MirrorMessage) -> Option<JsValue> {
    match serde_json::to_string(message) {
        Ok(json) => Some(JsValue::from_str(&json)),
        Err(err) => {
            error!("Could not encode a mirror message:", err.to_string());
            None
        }
    }
}

/// Decodes a `message` event, or `None` when it is not one of ours.
///
/// Origin first: the presenter view and its mirrors are pages of the same
/// server, so anything from elsewhere is not talking to us whatever it says.
/// A payload that is ours in origin but not in shape is logged at `debug` and
/// not `error`, because a page is posted at by extensions and dev tooling all
/// the time, and the console is where someone looks two minutes before a talk.
pub(crate) fn decode(event: &MessageEvent, origin: &str) -> Option<MirrorMessage> {
    if event.origin() != origin {
        return None;
    }
    let json = event.data().as_string()?;
    match serde_json::from_str::<MirrorMessage>(&json) {
        Ok(message) => Some(message),
        Err(err) => {
            debug!(
                "Ignoring a message that is not a mirror frame:",
                err.to_string()
            );
            None
        }
    }
}
