use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use toboggan_client::ConnectionStatus;
use toboggan_core::{Command, Notification, Slide, SlideId, TalkResponse};

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

/// The command that jumps to a slide the presenter typed.
///
/// They type the number printed on the slide, and `SlideId` is a 0-based index:
/// passing one straight to the other used to send every jump one slide too far.
pub(crate) fn goto_command(number: usize) -> Command {
    Command::GoTo {
        slide: SlideId::new(number.saturating_sub(1)),
    }
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

    /// The number on screen is 1-based; `SlideId` is not.
    #[test]
    fn typing_a_slide_number_goes_to_that_slide() {
        assert_eq!(
            goto_command(1),
            Command::GoTo {
                slide: SlideId::FIRST
            }
        );
        assert_eq!(
            goto_command(12),
            Command::GoTo {
                slide: SlideId::new(11)
            }
        );
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
