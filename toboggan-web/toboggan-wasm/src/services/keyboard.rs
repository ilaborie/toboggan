use std::collections::HashMap;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::{debug, error, info};
use gloo::events::{EventListener, EventListenerOptions};
use gloo::utils::window;
use toboggan_core::{Command, SlideId};
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::{
    BlankScreen, deck_keys_captured, show_goto_pending, toggle_blank, toggle_fullscreen,
    typing_into_editable,
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
    /// A digit of a slide number being typed.
    Digit(u8),
    /// Jump to the slide number typed so far.
    GotoTyped,
}

/// The largest slide number that can be typed.
///
/// Four digits is more slides than a talk has ever had, and the cap is what
/// stops a leaned-on key from overflowing the running multiplication.
const MAX_GOTO_TARGET: usize = 9_999;

/// The presenter types the number printed on the slide; `SlideId` is a 0-based
/// index.
fn goto_command(number: usize) -> Command {
    Command::GoTo {
        slide: SlideId::new(number.saturating_sub(1)),
    }
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
            // Typing a slide number. Digits accumulate and `Enter` jumps, so a
            // deck is not limited to the nine slides a single keystroke reaches.
            ("0", KeyAction::Digit(0)),
            ("1", KeyAction::Digit(1)),
            ("2", KeyAction::Digit(2)),
            ("3", KeyAction::Digit(3)),
            ("4", KeyAction::Digit(4)),
            ("5", KeyAction::Digit(5)),
            ("6", KeyAction::Digit(6)),
            ("7", KeyAction::Digit(7)),
            ("8", KeyAction::Digit(8)),
            ("9", KeyAction::Digit(9)),
            ("Enter", KeyAction::GotoTyped),
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
        // The slide number typed so far, if any.
        let mut pending_goto: Option<usize> = None;

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
                        // `Esc` is the help dialog's, which closes on it
                        // natively — so it is watched for without being
                        // consumed, only to drop a half-typed slide number.
                        if key == "Escape" && pending_goto.take().is_some() {
                            show_goto_pending(None);
                        }
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
                            // Navigating abandons a half-typed number rather
                            // than leaving it to land on the next `Enter`.
                            if pending_goto.take().is_some() {
                                show_goto_pending(None);
                            }
                            if tx.unbounded_send(command).is_err() {
                                error!("Failed to send keyboard action");
                            }
                        }
                        KeyAction::ToggleFullscreen => toggle_fullscreen(),
                        KeyAction::Blank(screen) => toggle_blank(screen),
                        KeyAction::Digit(digit) => {
                            let typed = pending_goto.unwrap_or(0) * 10 + usize::from(digit);
                            if typed <= MAX_GOTO_TARGET {
                                pending_goto = Some(typed);
                                show_goto_pending(Some(typed));
                            }
                        }
                        KeyAction::GotoTyped => {
                            if let Some(number) = pending_goto.take() {
                                show_goto_pending(None);
                                if tx.unbounded_send(goto_command(number)).is_err() {
                                    error!("Failed to send keyboard action");
                                }
                            }
                        }
                    }
                }
            });

        listener.forget();
        info!("⌨️ Keyboard service started");
    }
}
