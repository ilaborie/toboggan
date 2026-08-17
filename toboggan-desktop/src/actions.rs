use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use toboggan_core::Command;

/// Everything a keystroke can do in the desktop client.
///
/// One table, read by both the key handler and the help panel. They used to be
/// two: a `match` over twenty-odd arms in `app.rs` and twenty-eight hardcoded
/// strings in `views/help.rs` with nothing tying them together — so the panel
/// listed `Cmd+Q` for an arm that never existed and said nothing about `?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppAction {
    // Slide navigation
    First,
    Previous,
    Next,
    Last,
    // Step navigation
    PreviousStep,
    NextStep,
    // Presentation control
    Blink,
    // UI actions
    ToggleHelp,
    ToggleSidebar,
    ToggleFullscreen,
    CloseOverlay,
    Quit,
}

/// The groups the help panel prints, in order.
pub(crate) const HELP_GROUPS: &[(&str, &[AppAction])] = &[
    (
        "Step Navigation",
        &[AppAction::NextStep, AppAction::PreviousStep],
    ),
    (
        "Slide Navigation",
        &[
            AppAction::Next,
            AppAction::Previous,
            AppAction::First,
            AppAction::Last,
        ],
    ),
    ("Presentation", &[AppAction::Blink]),
    (
        "View",
        &[
            AppAction::ToggleHelp,
            AppAction::ToggleSidebar,
            AppAction::ToggleFullscreen,
            AppAction::CloseOverlay,
        ],
    ),
    ("Application", &[AppAction::Quit]),
];

impl AppAction {
    pub(crate) fn from_key(key: &Key, modifiers: Modifiers) -> Option<Self> {
        let action = match key {
            // PageDown/PageUp/Backspace are what a presenter remote emits, and
            // they drive *steps*: `NextStep` moves on to the next slide once a
            // slide's reveals run out, so the remote walks the whole deck,
            // whereas `NextSlide` would skip every reveal on the way.
            Key::Named(Named::Space | Named::ArrowDown | Named::PageDown) => Self::NextStep,
            Key::Named(Named::ArrowUp | Named::PageUp | Named::Backspace) => Self::PreviousStep,
            Key::Named(Named::ArrowRight) => Self::Next,
            Key::Named(Named::ArrowLeft) => Self::Previous,
            Key::Named(Named::Home) => Self::First,
            Key::Named(Named::End) => Self::Last,
            Key::Named(Named::F11) => Self::ToggleFullscreen,
            Key::Named(Named::Escape) => Self::CloseOverlay,
            Key::Character(character) if character == "q" && modifiers.command() => Self::Quit,
            Key::Character(character) => match character.as_str() {
                "h" | "H" | "?" => Self::ToggleHelp,
                "s" | "S" => Self::ToggleSidebar,
                "b" | "B" => Self::Blink,
                _ => return None,
            },
            _ => return None,
        };
        Some(action)
    }

    /// The command to send to the server, for the actions that drive the deck.
    pub(crate) fn command(self) -> Option<Command> {
        let command = match self {
            Self::First => Command::First,
            Self::Previous => Command::PreviousSlide,
            Self::Next => Command::NextSlide,
            Self::Last => Command::Last,
            Self::PreviousStep => Command::PreviousStep,
            Self::NextStep => Command::NextStep,
            Self::Blink => Command::Blink,
            Self::ToggleHelp
            | Self::ToggleSidebar
            | Self::ToggleFullscreen
            | Self::CloseOverlay
            | Self::Quit => return None,
        };
        Some(command)
    }

    /// Whether this action still runs while the help panel is up.
    ///
    /// The panel is modal: the deck's keys stand down behind it, so a presenter
    /// reading the shortcut list cannot walk the deck by accident. Only the keys
    /// that get them out of it again — and quitting — stay live.
    pub(crate) const fn ignores_help(self) -> bool {
        matches!(
            self,
            Self::ToggleHelp | Self::ToggleFullscreen | Self::CloseOverlay | Self::Quit
        )
    }

    pub(crate) const fn details(self) -> ActionDetails {
        match self {
            Self::First => ActionDetails::new(&["Home"], "First slide"),
            Self::Previous => ActionDetails::new(&["←"], "Previous slide"),
            Self::Next => ActionDetails::new(&["→"], "Next slide"),
            Self::Last => ActionDetails::new(&["End"], "Last slide"),
            Self::PreviousStep => {
                ActionDetails::new(&["↑", "PageUp", "Backspace"], "Previous step")
            }
            Self::NextStep => ActionDetails::new(&["↓", "Space", "PageDown"], "Next step"),
            Self::Blink => ActionDetails::new(&["b"], "Bell or blink"),
            Self::ToggleHelp => ActionDetails::new(&["h", "?"], "Toggle this help"),
            Self::ToggleSidebar => ActionDetails::new(&["s"], "Toggle sidebar"),
            Self::ToggleFullscreen => ActionDetails::new(&["F11"], "Toggle fullscreen"),
            Self::CloseOverlay => ActionDetails::new(&["Esc"], "Close help or error"),
            Self::Quit => ActionDetails::new(&["Cmd+Q"], "Quit"),
        }
    }
}

pub(crate) struct ActionDetails {
    pub(crate) keys: &'static [&'static str],
    pub(crate) description: &'static str,
}

impl ActionDetails {
    const fn new(keys: &'static [&'static str], description: &'static str) -> Self {
        Self { keys, description }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action_for(key: &Key) -> Option<AppAction> {
        AppAction::from_key(key, Modifiers::empty())
    }

    /// Every key the help panel advertises has to reach the handler, which is
    /// the whole reason the two now read the same table.
    #[test]
    fn every_documented_key_is_bound() {
        for (group, actions) in HELP_GROUPS {
            for action in *actions {
                for key in action.details().keys {
                    let pressed = match *key {
                        "Home" => Key::Named(Named::Home),
                        "End" => Key::Named(Named::End),
                        "←" => Key::Named(Named::ArrowLeft),
                        "→" => Key::Named(Named::ArrowRight),
                        "↑" => Key::Named(Named::ArrowUp),
                        "↓" => Key::Named(Named::ArrowDown),
                        "Space" => Key::Named(Named::Space),
                        "PageUp" => Key::Named(Named::PageUp),
                        "PageDown" => Key::Named(Named::PageDown),
                        "Backspace" => Key::Named(Named::Backspace),
                        "Esc" => Key::Named(Named::Escape),
                        "F11" => Key::Named(Named::F11),
                        // Modified and multi-key shortcuts carry their modifier
                        // in the label; they are checked on their own below.
                        label if label.contains('+') => continue,
                        character => Key::Character(character.into()),
                    };
                    assert_eq!(
                        action_for(&pressed),
                        Some(*action),
                        "{group}: {key} should map to {action:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn quit_needs_the_command_modifier() {
        let quit = Key::Character("q".into());
        assert_eq!(action_for(&quit), None);
        assert_eq!(
            AppAction::from_key(&quit, Modifiers::COMMAND),
            Some(AppAction::Quit)
        );
    }

    /// The deck's keys stand down behind the help panel; the ways out do not.
    #[test]
    fn only_the_ways_out_survive_the_help_panel() {
        assert!(!AppAction::NextStep.ignores_help());
        assert!(!AppAction::Blink.ignores_help());
        assert!(AppAction::CloseOverlay.ignores_help());
        assert!(AppAction::ToggleHelp.ignores_help());
    }
}
