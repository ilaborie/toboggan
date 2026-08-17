use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::{EventListener, EventListenerOptions};
use gloo::utils::window;
use toboggan_core::Command;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::{deck_keys_captured, typing_into_editable};

#[derive(Debug, Clone)]
pub(crate) struct KeyboardMapping(HashMap<&'static str, Command>);

impl Default for KeyboardMapping {
    fn default() -> Self {
        let mapping = HashMap::from([
            ("ArrowLeft", Command::PreviousSlide),
            ("ArrowUp", Command::PreviousStep),
            ("ArrowRight", Command::NextSlide),
            ("ArrowDown", Command::NextStep),
            (" ", Command::NextStep),
            // What a presenter remote emits. Bound to the *step* commands, not
            // the slide ones: `NextStep` moves on to the next slide once a
            // slide's reveals run out, so the remote walks the whole deck,
            // whereas `NextSlide` would make every reveal unreachable from it.
            ("PageDown", Command::NextStep),
            ("PageUp", Command::PreviousStep),
            ("Backspace", Command::PreviousStep),
            ("Home", Command::First),
            ("End", Command::Last),
            ("b", Command::Blink),
            ("B", Command::Blink),
        ]);
        Self(mapping)
    }
}

impl KeyboardMapping {
    pub(crate) fn get(&self, key: &str) -> Option<Command> {
        self.0.get(key).cloned()
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&'static str, &Command)> {
        self.0.iter().map(|(key, cmd)| (*key, cmd))
    }
}

pub(crate) struct KeyboardService {
    tx: UnboundedSender<Command>,
    mapping: KeyboardMapping,
}

impl KeyboardService {
    pub(crate) fn new(tx: UnboundedSender<Command>, mapping: KeyboardMapping) -> Self {
        Self { tx, mapping }
    }

    pub(crate) fn start(&mut self) {
        let tx = self.tx.clone();
        let mapping = self.mapping.clone();

        // Every key the deck binds already means something to the browser:
        // space and the arrows scroll, PageUp/PageDown page, Backspace used to
        // navigate back. gloo registers passive listeners unless asked not to,
        // and a passive listener's `preventDefault` is ignored — so without
        // this the page scrolls out from under the slide as the deck advances.
        let options = EventListenerOptions::enable_prevent_default();
        let listener =
            EventListener::new_with_options(&window(), "keydown", options, move |event| {
                if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
                    // The deck's bindings are bare keys — `space`, the arrows — which
                    // are also just characters someone may be typing. This handler
                    // sits on `window` and used to fire for every one of them, so a
                    // `space` at the quake terminal's shell prompt advanced the
                    // slide as well, and the slide change restarted the session the
                    // presenter was working in.
                    if deck_keys_captured() || typing_into_editable(keyboard_event) {
                        return;
                    }
                    let key = keyboard_event.key();
                    if let Some(action) = mapping.get(&key) {
                        keyboard_event.prevent_default();
                        if tx.unbounded_send(action).is_err() {
                            error!("Failed to send keyboard action");
                        }
                    } else {
                        debug!("No mapping for key:", &key);
                    }
                }
            });

        listener.forget();
        info!("⌨️ Keyboard service started");
    }
}
