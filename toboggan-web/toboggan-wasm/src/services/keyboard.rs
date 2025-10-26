use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::EventListener;
use gloo::utils::window;
use toboggan_core::Command;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::Action;

#[derive(Debug, Clone)]
pub struct KeyboardMapping(HashMap<&'static str, Action>);

impl Default for KeyboardMapping {
    fn default() -> Self {
        let mapping = HashMap::from([
            ("ArrowLeft", Action::Command(Command::Previous)),
            ("ArrowUp", Action::PreviousStep),
            ("ArrowRight", Action::Command(Command::Next)),
            ("ArrowDown", Action::NextStep),
            (" ", Action::NextStep),
            ("Home", Action::Command(Command::First)),
            ("End", Action::Command(Command::Last)),
            ("p", Action::Command(Command::Pause)),
            ("P", Action::Command(Command::Pause)),
            ("r", Action::Command(Command::Resume)),
            ("R", Action::Command(Command::Resume)),
            ("b", Action::Command(Command::Blink)),
            ("B", Action::Command(Command::Blink)),
        ]);
        Self(mapping)
    }
}

impl KeyboardMapping {
    pub fn get(&self, key: &str) -> Option<Action> {
        self.0.get(key).cloned()
    }
}

pub struct KeyboardService {
    tx: UnboundedSender<Action>,
    mapping: KeyboardMapping,
}

impl KeyboardService {
    pub fn new(tx: UnboundedSender<Action>, mapping: KeyboardMapping) -> Self {
        Self { tx, mapping }
    }

    pub fn start(&mut self) {
        let tx = self.tx.clone();
        let mapping = self.mapping.clone();

        let listener = EventListener::new(&window(), "keydown", move |event| {
            if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
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
