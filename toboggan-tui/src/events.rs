use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use toboggan_client::ConnectionStatus;
use toboggan_core::{Command, Notification, Slide, SlideId, TalkResponse};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,

    // Could refactor to use toboggan_client::CommunicationMessage for consistency
    NotificationReceived(Notification),
    ConnectionStatus(ConnectionStatus),
    TalkAndSlidesRefetched(Box<TalkResponse>, Vec<Slide>),
    Error(String),
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum AppAction {
    // Slide navigation
    First,
    Previous,
    Next,
    Last,
    #[display("Slide {_0}")]
    Goto(u8),
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
            // Step navigation: Space, Down, Up
            KeyCode::Down | KeyCode::Char(' ') => Self::NextStep,
            KeyCode::Up => Self::PreviousStep,
            // Slide navigation: Left, Right
            KeyCode::Left => Self::Previous,
            KeyCode::Right => Self::Next,
            KeyCode::Home => Self::First,
            KeyCode::End => Self::Last,
            KeyCode::Char('b' | 'B') => Self::Blink,
            KeyCode::Char(ch @ '1'..='9') =>
            {
                #[allow(clippy::expect_used)]
                Self::Goto(ch.to_string().parse().expect("1..=9 should parse"))
            }
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
            Self::Goto(_) => "1..n",
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
            Self::Goto(_) => ActionDetails::new(vec!["1..n"], "Go to slide n"),
            Self::PreviousStep => ActionDetails::new(vec!["↑"], "Previous step"),
            Self::NextStep => ActionDetails::new(vec!["↓", "Space"], "Next step"),
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
            Self::Goto(id) => Command::GoTo {
                slide: SlideId::new(usize::from(id)),
            },
            Self::ShowLog | Self::Close | Self::Quit | Self::Help => {
                return None;
            }
        };
        Some(cmd)
    }
}

pub struct ActionDetails {
    pub(crate) keys: Vec<&'static str>,
    pub(crate) description: &'static str,
}

impl ActionDetails {
    pub fn new(keys: Vec<&'static str>, description: &'static str) -> Self {
        Self { keys, description }
    }
}
