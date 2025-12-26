use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::EventListener;
use gloo::utils::window;
use toboggan_core::Command;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

#[derive(Debug, Clone)]
pub struct KeyboardMapping(HashMap<&'static str, Command>);

impl Default for KeyboardMapping {
    fn default() -> Self {
        let mapping = HashMap::from([
            ("ArrowLeft", Command::Previous),
            ("ArrowUp", Command::PreviousStep),
            ("ArrowRight", Command::Next),
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
    pub fn get(&self, key: &str) -> Option<Command> {
        self.0.get(key).cloned()
    }
}

pub struct KeyboardService {
    tx: UnboundedSender<Command>,
    mapping: KeyboardMapping,
}

impl KeyboardService {
    pub fn new(tx: UnboundedSender<Command>, mapping: KeyboardMapping) -> Self {
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
