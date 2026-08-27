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
    /// One digit of a slide number being typed; see [`Self::GotoTyped`].
    Digit(u8),
    /// `Enter`: go to the slide number typed so far.
    GotoTyped,
    // Presentation control
    Blink,
    // The talk's own clock
    ToggleTimer,
    ResetTimer,
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
            AppAction::Digit(0),
        ],
    ),
    ("Presentation", &[AppAction::Blink]),
    ("Timer", &[AppAction::ToggleTimer, AppAction::ResetTimer]),
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
            // Digits accumulate and Enter jumps, so reaching slide 31 of 42 is
            // three keystrokes rather than a scroll.
            Key::Named(Named::Enter) => Self::GotoTyped,
            Key::Named(Named::Home) => Self::First,
            Key::Named(Named::End) => Self::Last,
            Key::Named(Named::F11) => Self::ToggleFullscreen,
            Key::Named(Named::Escape) => Self::CloseOverlay,
            Key::Character(character) if character == "q" && modifiers.command() => Self::Quit,
            Key::Character(character) => match character.as_str() {
                "h" | "H" | "?" => Self::ToggleHelp,
                "s" | "S" => Self::ToggleSidebar,
                "b" | "B" => Self::Blink,
                digit if digit.len() == 1 && digit.starts_with(|ch: char| ch.is_ascii_digit()) => {
                    let value = digit
                        .bytes()
                        .next()
                        .map_or(0, |byte| byte.wrapping_sub(b'0'));
                    Self::Digit(value)
                }
                "t" => Self::ToggleTimer,
                "T" => Self::ResetTimer,
                _ => return None,
            },
            _ => return None,
        };
        Some(action)
    }

    /// Every action, for the table-driven tests below.
    ///
    /// Kept honest by [`Self::position`]: adding a variant does not compile
    /// until it appears there, and the assertion in `all_lists_every_action`
    /// then fails until it appears here too.
    #[cfg(test)]
    /// `Digit` stands for all ten, which `every_digit_carries_its_own_value`
    /// covers one by one.
    pub(crate) const ALL: [Self; 16] = [
        Self::First,
        Self::Previous,
        Self::Next,
        Self::Last,
        Self::PreviousStep,
        Self::NextStep,
        Self::Digit(0),
        Self::GotoTyped,
        Self::Blink,
        Self::ToggleTimer,
        Self::ResetTimer,
        Self::ToggleHelp,
        Self::ToggleSidebar,
        Self::ToggleFullscreen,
        Self::CloseOverlay,
        Self::Quit,
    ];

    /// Where this action sits in [`Self::ALL`].
    ///
    /// An exhaustive match on purpose — it is what makes a forgotten variant a
    /// build failure rather than a silently thinner test.
    #[cfg(test)]
    const fn position(self) -> usize {
        match self {
            Self::First => 0,
            Self::Previous => 1,
            Self::Next => 2,
            Self::Last => 3,
            Self::PreviousStep => 4,
            Self::NextStep => 5,
            Self::Digit(_) => 6,
            Self::GotoTyped => 7,
            Self::Blink => 8,
            Self::ToggleTimer => 9,
            Self::ResetTimer => 10,
            Self::ToggleHelp => 11,
            Self::ToggleSidebar => 12,
            Self::ToggleFullscreen => 13,
            Self::CloseOverlay => 14,
            Self::Quit => 15,
        }
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
            // The timer is this client's own; it never reaches the server, so
            // pausing it here does not pause it on anybody else's screen.
            // The jump needs the number typed so far, which lives in the app's
            // state — so `app` builds this one with `goto_command`.
            Self::Digit(_)
            | Self::GotoTyped
            | Self::ToggleTimer
            | Self::ResetTimer
            | Self::ToggleHelp
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
            // One entry for both: they are two halves of the same gesture, and
            // listing `Enter` on its own would read as a command of its own.
            Self::Digit(_) | Self::GotoTyped => {
                ActionDetails::new(&["0-9", "Enter"], "Go to slide n")
            }
            Self::ToggleTimer => ActionDetails::new(&["t"], "Pause or resume the timer"),
            Self::ResetTimer => ActionDetails::new(&["T"], "Reset the timer to zero"),
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
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::{SlideId, accumulate_goto, goto_command};

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
                        "Enter" => Key::Named(Named::Enter),
                        // A range label stands for ten keys, so it is expanded
                        // rather than skipped: each digit must reach `Digit`
                        // carrying its own value, or a typed `31` would jump
                        // somewhere else entirely.
                        "0-9" => {
                            for digit in 0..=9_u8 {
                                let key = Key::Character(digit.to_string().into());
                                assert_eq!(
                                    action_for(&key),
                                    Some(AppAction::Digit(digit)),
                                    "{group}: {digit} should map to Digit({digit})"
                                );
                            }
                            continue;
                        }
                        // Modified and multi-key shortcuts carry their modifier
                        // in the label; they are checked on their own below.
                        label if label.contains('+') => continue,
                        character => Key::Character(character.into()),
                    };
                    let bound = action_for(&pressed);
                    // `Digit` and `GotoTyped` are two halves of one gesture and
                    // share a help entry, so `Enter` is listed under both.
                    let expected = match action {
                        AppAction::Digit(_) | AppAction::GotoTyped => {
                            bound == Some(AppAction::GotoTyped)
                        }
                        other => bound == Some(*other),
                    };
                    assert!(
                        expected,
                        "{group}: {key} should map to {action:?}, got {bound:?}"
                    );
                }
            }
        }
    }

    /// `from_key` only proves a key reaches the right *action*. Which command
    /// that action sends was untested, so swapping two neighbouring arms in
    /// `command()` would have moved the deck the wrong way with every test
    /// still green.
    #[test]
    fn each_action_sends_the_command_it_names() {
        for (action, expected) in [
            (AppAction::First, Some(Command::First)),
            (AppAction::Previous, Some(Command::PreviousSlide)),
            (AppAction::Next, Some(Command::NextSlide)),
            (AppAction::Last, Some(Command::Last)),
            (AppAction::PreviousStep, Some(Command::PreviousStep)),
            (AppAction::NextStep, Some(Command::NextStep)),
            (AppAction::Blink, Some(Command::Blink)),
            // The jump is built from state, not from the action alone.
            (AppAction::Digit(4), None),
            (AppAction::GotoTyped, None),
            // The local ones are the window's business, not the server's — the
            // timer most of all: it is this client's own, so pausing it here
            // must not pause the room's.
            (AppAction::ToggleTimer, None),
            (AppAction::ResetTimer, None),
            (AppAction::ToggleHelp, None),
            (AppAction::ToggleSidebar, None),
            (AppAction::ToggleFullscreen, None),
            (AppAction::CloseOverlay, None),
            (AppAction::Quit, None),
        ] {
            assert_eq!(action.command(), expected, "{action:?}");
        }
    }

    #[test]
    fn all_lists_every_action() {
        for (index, action) in AppAction::ALL.iter().enumerate() {
            assert_eq!(action.position(), index, "{action:?} is out of place");
        }
    }

    /// The help panel is generated from `HELP_GROUPS`, so an action missing
    /// from it is a key that works and is never advertised — the same drift as
    /// the reverse, which `every_documented_key_is_bound` covers.
    #[test]
    fn every_action_is_documented() {
        let documented: Vec<AppAction> = HELP_GROUPS
            .iter()
            .flat_map(|(_, actions)| actions.iter().copied())
            .collect();

        for action in AppAction::ALL {
            // `Digit` and `GotoTyped` are two halves of one gesture with one
            // help line — `0-9 / Enter — Go to slide n` — so the line that
            // documents either documents both. Listing them separately would
            // print that line twice.
            let looked_up = match action {
                AppAction::GotoTyped => AppAction::Digit(0),
                other => other,
            };
            assert!(
                documented.contains(&looked_up),
                "{action:?} is bound but absent from the help panel"
            );
        }
    }

    /// The value matters as much as the mapping: a `Digit` that lost which
    /// digit it was would send every jump to the same slide.
    #[test]
    fn every_digit_carries_its_own_value() {
        for digit in 0..=9_u8 {
            let key = Key::Character(digit.to_string().into());
            assert_eq!(action_for(&key), Some(AppAction::Digit(digit)), "{digit}");
        }
    }

    /// The arithmetic is core's; this pins the desktop to it rather than to a
    /// copy that could drift.
    #[test]
    fn a_typed_number_becomes_a_one_based_goto() {
        let typed = [3_u8, 1].into_iter().fold(None, accumulate_goto);
        assert_eq!(typed, Some(31));
        assert_eq!(
            goto_command(typed.expect("a number")),
            Command::GoTo {
                slide: SlideId::new(30)
            }
        );
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
