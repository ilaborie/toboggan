use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info, warn};
use gloo::events::EventListener;
use gloo::utils::window;
use toboggan_core::Command;
use wasm_bindgen::JsCast;
use web_sys::{Event, KeyboardEvent};

#[derive(Debug, Clone)]
pub struct KeyboardMapping(HashMap<&'static str, Command>);

impl Default for KeyboardMapping {
    fn default() -> Self {
        let mapping = HashMap::from([
            ("ArrowLeft", Command::Previous),
            ("ArrowUp", Command::Previous),
            ("ArrowRight", Command::Next),
            ("ArrowDown", Command::Next),
            (" ", Command::Next),
            ("Home", Command::First),
            ("End", Command::Last),
            ("p", Command::Pause),
            ("P", Command::Pause),
            ("r", Command::Resume),
            ("R", Command::Resume),
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
    tx: Option<UnboundedSender<Command>>,
    mapping: KeyboardMapping,
    // We'll leak the listener with forget() to keep it alive
}

impl KeyboardService {
    pub fn new(tx: UnboundedSender<Command>, mapping: KeyboardMapping) -> Self {
        let tx = Some(tx);

        Self { tx, mapping }
    }

    pub fn start(&mut self) {
        let Some(tx) = &self.tx else {
            warn!("Illegal state, no tx found");
            return;
        };

        // Use window for global keyboard capture
        let win = window();

        let tx = tx.clone();
        let mapping = self.mapping.clone();

        // Create event listener with gloo
        let listener = EventListener::new(&win, "keydown", move |event| {
            handle_keydown(event, &tx, &mapping);
        });

        // Forget the listener to keep it alive for the lifetime of the application
        listener.forget();
        info!("⌨️ Keyboard service attached to window and listening for events");
    }
}

fn handle_keydown(event: &Event, tx: &UnboundedSender<Command>, mapping: &KeyboardMapping) {
    let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() else {
        warn!("⌨️ Event is not a KeyboardEvent");
        return;
    };

    let key = keyboard_event.key();
    // debug!("⌨️ Key pressed:", &key);

    // Check if we have a mapping for this key
    let Some(cmd) = mapping.get(&key) else {
        debug!("⌨️ No mapping found for key:", &key);
        return;
    };

    // Send the command
    if let Err(err) = tx.unbounded_send(cmd) {
        error!("Failed to send keyboard command:", err.to_string());
    }
}

impl Drop for KeyboardService {
    fn drop(&mut self) {
        // Clean up sender - listener was forgotten (leaked) to keep it alive
        self.tx.take();
    }
}
