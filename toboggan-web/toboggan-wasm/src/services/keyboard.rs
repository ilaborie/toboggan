use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::{EventListener, EventListenerOptions};
use gloo::utils::window;
use toboggan_core::Command;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::{
    BlankScreen, deck_keys_captured, toggle_blank, toggle_fullscreen, typing_into_editable,
};

/// What a key does.
///
/// Not every binding is a server command. Fullscreen and a blank screen belong
/// to the screen in front of the presenter, not to the presentation, and the
/// server has no notion of either — so a mapping that could only hold a
/// [`Command`] could not express them at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyAction {
    /// Sent to the server, which broadcasts the result to every client.
    Send(Command),
    /// Handled in this tab.
    ToggleFullscreen,
    /// Handled in this tab.
    Blank(BlankScreen),
}

#[derive(Debug, Clone)]
pub(crate) struct KeyboardMapping(HashMap<&'static str, KeyAction>);

impl Default for KeyboardMapping {
    fn default() -> Self {
        let mapping = HashMap::from([
            ("ArrowLeft", KeyAction::Send(Command::PreviousSlide)),
            ("ArrowUp", KeyAction::Send(Command::PreviousStep)),
            ("ArrowRight", KeyAction::Send(Command::NextSlide)),
            ("ArrowDown", KeyAction::Send(Command::NextStep)),
            (" ", KeyAction::Send(Command::NextStep)),
            // What a presenter remote emits. Bound to the *step* commands, not
            // the slide ones: `NextStep` moves on to the next slide once a
            // slide's reveals run out, so the remote walks the whole deck,
            // whereas `NextSlide` would make every reveal unreachable from it.
            ("PageDown", KeyAction::Send(Command::NextStep)),
            ("PageUp", KeyAction::Send(Command::PreviousStep)),
            ("Backspace", KeyAction::Send(Command::PreviousStep)),
            ("Home", KeyAction::Send(Command::First)),
            ("End", KeyAction::Send(Command::Last)),
            ("b", KeyAction::Send(Command::Blink)),
            ("B", KeyAction::Send(Command::Blink)),
            ("f", KeyAction::ToggleFullscreen),
            ("F", KeyAction::ToggleFullscreen),
            (".", KeyAction::Blank(BlankScreen::Black)),
            ("w", KeyAction::Blank(BlankScreen::White)),
            ("W", KeyAction::Blank(BlankScreen::White)),
        ]);
        Self(mapping)
    }
}

impl KeyboardMapping {
    pub(crate) fn get(&self, key: &str) -> Option<KeyAction> {
        self.0.get(key).cloned()
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&'static str, &KeyAction)> {
        self.0.iter().map(|(key, action)| (*key, action))
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
                    let Some(action) = mapping.get(&key) else {
                        debug!("No mapping for key:", &key);
                        return;
                    };
                    keyboard_event.prevent_default();
                    // The local actions run right here rather than going out
                    // over the channel: a browser only grants fullscreen off a
                    // user gesture, and a task that runs after this handler
                    // returns no longer has one.
                    match action {
                        KeyAction::Send(command) => {
                            if tx.unbounded_send(command).is_err() {
                                error!("Failed to send keyboard action");
                            }
                        }
                        KeyAction::ToggleFullscreen => toggle_fullscreen(),
                        KeyAction::Blank(screen) => toggle_blank(screen),
                    }
                }
            });

        listener.forget();
        info!("⌨️ Keyboard service started");
    }
}
