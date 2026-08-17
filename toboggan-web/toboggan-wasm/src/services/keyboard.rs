use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::EventListener;
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

        let listener = EventListener::new(&window(), "keydown", move |event| {
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
