//! A deck rendered inside one of the presenter view's panes.
//!
//! It is the whole `/run` page — [`mount_deck`] builds the same DOM, the deck's
//! `_head.html` lands in this document's head, the slide is laid out against
//! this document's viewport — with everything that talks to the world taken
//! away. No socket, no commands, no keyboard, no terminals, no toasts, no help
//! dialog. It paints what it is handed and nothing else.
//!
//! That is also what makes it safe to frame the deck on the presenter's own
//! machine: the server grants a loopback client the presenter role when it
//! registers, and a mirror never registers.
//!
//! See [`crate::services::mirror`] for why the panes are iframes at all.

use std::cell::RefCell;
use std::rc::Rc;

use gloo::console::info;
use gloo::events::EventListener;
use gloo::utils::window;
use wasm_bindgen::JsCast as _;
use web_sys::{HtmlElement, MessageEvent};

use crate::components::WasmElement;
use crate::components::deck::DeckPainter;
use crate::services::mirror::{self, MirrorMessage, MirrorPane};

pub(crate) struct MirrorApp {
    pane: MirrorPane,
}

impl MirrorApp {
    pub(crate) const fn new(pane: MirrorPane) -> Self {
        Self { pane }
    }
}

impl WasmElement for MirrorApp {
    fn render(&mut self, host: &HtmlElement) {
        let inner = Rc::new(RefCell::new(DeckPainter::mount(host)));

        let Some(origin) = mirror::page_origin() else {
            return;
        };

        // Deliberately leaked, the way the keyboard service leaks its own.
        // A mirror is the whole page; there is nothing to tear down, and the
        // listener has to outlive this call — held on `self`, it went out of
        // scope with the `MirrorApp` the moment `start_mirror_app` returned,
        // which unhooked the listener and left the pane blank for ever.
        EventListener::new(&window(), "message", move |event| {
            let Some(event) = event.dyn_ref::<MessageEvent>() else {
                return;
            };
            match mirror::decode(event, &origin) {
                Some(MirrorMessage::Frame(frame)) => inner.borrow_mut().show(*frame),
                // A mirror never hears another mirror's handshake. The arm
                // exists because one type serves both directions, which is what
                // keeps the two sides from drifting.
                Some(MirrorMessage::Ready { .. }) | None => (),
            }
        })
        .forget();

        // After the listener, so a frame answering this hello cannot arrive
        // before there is anything to receive it.
        announce_ready(self.pane);
    }
}

/// Tells the presenter view this pane can accept frames.
///
/// It has to be asked for: the presenter has state before its iframes finish
/// loading — it came over a socket that was open before the `<iframe>` existed
/// — and a frame posted into a document still on `about:blank` is delivered to
/// *that* document and thrown away with it.
fn announce_ready(pane: MirrorPane) {
    // A mirror opened directly in a tab has no parent, or is its own. Posting
    // to itself would be harmless — it ignores `Ready` — but saying so is the
    // more useful thing to leave in the console for whoever typed the URL.
    let Ok(Some(parent)) = window()
        .parent()
        .map(|parent| parent.filter(|frame| *frame != window()))
    else {
        info!("A mirror is meant to be framed by the presenter view");
        return;
    };

    let (Some(origin), Some(message)) = (
        mirror::page_origin(),
        mirror::encode(&MirrorMessage::Ready { pane }),
    ) else {
        return;
    };
    if let Err(err) = parent.post_message(&message, &origin) {
        gloo::console::error!("A mirror's hello was refused:", err);
    }
}
