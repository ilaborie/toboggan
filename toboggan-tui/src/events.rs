use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use toboggan_client::ConnectionStatus;
use toboggan_core::{Command, Notification, Slide, TalkResponse};

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Key(KeyEvent),
    Tick,

    // Could refactor to use toboggan_client::CommunicationMessage for consistency
    NotificationReceived(Notification),
    ConnectionStatus(ConnectionStatus),
    TalkAndSlidesRefetched(Box<TalkResponse>, Vec<Slide>),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
pub(crate) enum AppAction {
    // Slide navigation
    First,
    Previous,
    Next,
    Last,
    #[display("Slide {_0}")]
    Digit(u8),
    #[display("Go to slide")]
    GotoTyped,
    // Step navigation
    PreviousStep,
    NextStep,
    // Presentation control
    #[display("♪")]
    Blink,
    // UI actions
    #[display("Show log")]
    ShowLog,
    Close,
    Quit,
    Help,
}

impl AppAction {
    /// Every action, for the table-driven tests.
    ///
    /// `Digit` stands for all ten, which are covered one by one.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 13] = [
        Self::First,
        Self::Previous,
        Self::Next,
        Self::Last,
        Self::Digit(0),
        Self::GotoTyped,
        Self::PreviousStep,
        Self::NextStep,
        Self::Blink,
        Self::ShowLog,
        Self::Close,
        Self::Quit,
        Self::Help,
    ];

    pub(crate) fn from_key(event: KeyEvent) -> Option<Self> {
        let action = match event.code {
            KeyCode::Char('q' | 'Q') => Self::Quit,
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => Self::Quit,
            KeyCode::Char('h' | 'H' | '?') => Self::Help,
            // Step navigation: Space, Down, Up — plus what a presenter remote
            // emits. The remote drives *steps*, not slides: `NextStep` moves on
            // to the next slide once a slide's reveals run out, so it walks the
            // whole deck, whereas `NextSlide` would skip every reveal.
            KeyCode::Down | KeyCode::Char(' ') | KeyCode::PageDown => Self::NextStep,
            KeyCode::Up | KeyCode::PageUp | KeyCode::Backspace => Self::PreviousStep,
            // Slide navigation: Left, Right
            KeyCode::Left => Self::Previous,
            KeyCode::Right => Self::Next,
            KeyCode::Home => Self::First,
            KeyCode::End => Self::Last,
            KeyCode::Char('b' | 'B') => Self::Blink,
            // Digits accumulate and Enter jumps, so a deck is not limited to
            // the nine slides a single keystroke can reach.
            KeyCode::Char(ch @ '0'..='9') => Self::Digit(digit_of(ch)),
            KeyCode::Enter => Self::GotoTyped,
            KeyCode::Char('l' | 'L') => Self::ShowLog,
            KeyCode::Esc => Self::Close,
            _ => {
                return None;
            }
        };
        Some(action)
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::First => "Home",
            Self::Previous => "←",
            Self::Next => "→",
            Self::Last => "End",
            Self::Digit(_) | Self::GotoTyped => "0..9",
            Self::PreviousStep => "↑",
            Self::NextStep => "↓",
            Self::Blink => "b",
            Self::ShowLog => "l",
            Self::Close => "Esc",
            Self::Quit => "q",
            Self::Help => "?",
        }
    }

    pub(crate) fn details(self) -> ActionDetails {
        match self {
            Self::First => ActionDetails::new(vec!["Home"], "Go to first slide"),
            Self::Previous => ActionDetails::new(vec!["←"], "Previous slide"),
            Self::Next => ActionDetails::new(vec!["→"], "Next slide"),
            Self::Last => ActionDetails::new(vec!["End"], "Go to last slide"),
            Self::Digit(_) | Self::GotoTyped => {
                ActionDetails::new(vec!["0..9", "Enter"], "Go to slide n")
            }
            Self::PreviousStep => ActionDetails::new(vec!["↑", "PgUp", "Bksp"], "Previous step"),
            Self::NextStep => ActionDetails::new(vec!["↓", "Space", "PgDn"], "Next step"),
            Self::Blink => ActionDetails::new(vec!["b", "B"], "Bell or Blink"),
            Self::ShowLog => ActionDetails::new(vec!["l", "L"], "Show logs"),
            Self::Close => ActionDetails::new(vec!["Esc"], "Close popup"),
            Self::Quit => ActionDetails::new(vec!["q", "Q", "Ctrl-c"], "Quit"),
            Self::Help => ActionDetails::new(vec!["?", "h", "H"], "Show help"),
        }
    }

    pub(crate) fn command(self) -> Option<Command> {
        let cmd = match self {
            Self::First => Command::First,
            Self::Previous => Command::PreviousSlide,
            Self::Next => Command::NextSlide,
            Self::Last => Command::Last,
            Self::PreviousStep => Command::PreviousStep,
            Self::NextStep => Command::NextStep,
            Self::Blink => Command::Blink,
            // The typed slide number lives in `AppState`, which turns it into a
            // `GoTo` once `Enter` closes it — a single digit is not a command.
            Self::Digit(_)
            | Self::GotoTyped
            | Self::ShowLog
            | Self::Close
            | Self::Quit
            | Self::Help => {
                return None;
            }
        };
        Some(cmd)
    }
}

/// The value of an ASCII digit, or `0` for anything else — which
/// [`AppAction::from_key`] has already ruled out.
fn digit_of(character: char) -> u8 {
    u8::try_from(character.to_digit(10).unwrap_or(0)).unwrap_or(0)
}

pub(crate) struct ActionDetails {
    pub(crate) keys: Vec<&'static str>,
    pub(crate) description: &'static str,
}

impl ActionDetails {
    pub(crate) fn new(keys: Vec<&'static str>, description: &'static str) -> Self {
        Self { keys, description }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_for(code: KeyCode) -> Option<Command> {
        AppAction::from_key(KeyEvent::from(code))?.command()
    }

    /// A presenter remote sends PageUp/PageDown, and it has no other buttons:
    /// bound to whole slides it could never reach a single reveal.
    #[test]
    fn a_remote_walks_the_deck_a_step_at_a_time() {
        for code in [KeyCode::PageDown, KeyCode::Char(' '), KeyCode::Down] {
            assert_eq!(command_for(code), Some(Command::NextStep), "{code:?}");
        }
        for code in [KeyCode::PageUp, KeyCode::Backspace, KeyCode::Up] {
            assert_eq!(command_for(code), Some(Command::PreviousStep), "{code:?}");
        }
    }

    /// A digit is not a command on its own: it is one keystroke of a number
    /// that only becomes a jump when `Enter` closes it.
    #[test]
    fn a_digit_carries_its_value_and_sends_nothing() {
        assert_eq!(
            AppAction::from_key(KeyEvent::from(KeyCode::Char('7'))),
            Some(AppAction::Digit(7))
        );
        assert_eq!(command_for(KeyCode::Char('7')), None);
        assert_eq!(
            AppAction::from_key(KeyEvent::from(KeyCode::Enter)),
            Some(AppAction::GotoTyped)
        );
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    fn action_for(code: KeyCode) -> Option<AppAction> {
        AppAction::from_key(KeyEvent::from(code))
    }

    /// The keys that move the deck between slides, as opposed to through the
    /// reveals on one. Three of thirteen bindings were covered before; a
    /// swapped arm here sends the presenter the wrong way with nothing failing.
    #[test]
    fn the_arrows_and_ends_move_between_slides() {
        assert_eq!(action_for(KeyCode::Left), Some(AppAction::Previous));
        assert_eq!(action_for(KeyCode::Right), Some(AppAction::Next));
        assert_eq!(action_for(KeyCode::Home), Some(AppAction::First));
        assert_eq!(action_for(KeyCode::End), Some(AppAction::Last));
    }

    /// Both cases, because a presenter with caps lock on is still a presenter.
    #[test]
    fn the_letter_keys_are_case_insensitive() {
        for (lower, upper, action) in [
            ('q', 'Q', AppAction::Quit),
            ('h', 'H', AppAction::Help),
            ('b', 'B', AppAction::Blink),
            ('l', 'L', AppAction::ShowLog),
        ] {
            assert_eq!(action_for(KeyCode::Char(lower)), Some(action), "{lower}");
            assert_eq!(action_for(KeyCode::Char(upper)), Some(action), "{upper}");
        }
        assert_eq!(action_for(KeyCode::Char('?')), Some(AppAction::Help));
    }

    /// `Ctrl-C` has to quit even though a bare `c` is not bound: it is what
    /// someone reaches for when a terminal UI stops responding.
    #[test]
    fn ctrl_c_quits_and_a_bare_c_does_nothing() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(AppAction::from_key(ctrl_c), Some(AppAction::Quit));
        assert_eq!(action_for(KeyCode::Char('c')), None);
    }

    #[test]
    fn escape_closes_and_enter_jumps() {
        assert_eq!(action_for(KeyCode::Esc), Some(AppAction::Close));
        assert_eq!(action_for(KeyCode::Enter), Some(AppAction::GotoTyped));
    }

    /// Every digit, because the map converts the character and an off-by-one in
    /// that conversion would misroute every jump.
    #[test]
    fn every_digit_carries_its_own_value() {
        for digit in 0..=9u8 {
            let key = KeyCode::Char(char::from(b'0' + digit));
            assert_eq!(action_for(key), Some(AppAction::Digit(digit)), "{digit}");
        }
    }

    /// An unbound key must fall through rather than being absorbed, or the
    /// terminal underneath never sees it.
    #[test]
    fn an_unbound_key_is_not_claimed() {
        for code in [KeyCode::Tab, KeyCode::Char('z'), KeyCode::Delete] {
            assert_eq!(action_for(code), None, "{code:?}");
        }
    }

    /// The help panel is built from `key()` and `details()`, so a key it names
    /// has to be one the handler answers to — the same drift the desktop
    /// client's `every_documented_key_is_bound` guards against.
    #[test]
    fn every_advertised_key_is_bound() {
        for action in AppAction::ALL {
            let key = match action.key() {
                "Home" => KeyCode::Home,
                "End" => KeyCode::End,
                "←" => KeyCode::Left,
                "→" => KeyCode::Right,
                "↑" => KeyCode::Up,
                "↓" => KeyCode::Down,
                "Space" => KeyCode::Char(' '),
                "PageUp" => KeyCode::PageUp,
                "PageDown" => KeyCode::PageDown,
                "Enter" => KeyCode::Enter,
                "Esc" => KeyCode::Esc,
                // Digits are one binding per character, covered above, and
                // `Ctrl-C` carries a modifier its label cannot express.
                other if other.len() != 1 => continue,
                other => KeyCode::Char(other.chars().next().unwrap_or('\0')),
            };
            assert!(
                AppAction::from_key(KeyEvent::from(key)).is_some(),
                "{action:?} advertises {} but nothing is bound to it",
                action.key()
            );
        }
    }
}
