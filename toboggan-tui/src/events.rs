use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use toboggan_client::ConnectionStatus;
use toboggan_core::{Command, Notification};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,

    // TODO use CommunicationMessage
    NotificationReceived(Notification),
    ConnectionStatus(ConnectionStatus),
    Error(String),
}

#[derive(Debug, Clone, Copy, derive_more::Display)]
pub enum AppAction {
    First,
    Previous,
    Next,
    Last,
    Pause,
    #[display("Slide {_0}")]
    Goto(u8),
    Resume,
    #[display("♪")]
    Blink,

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
            KeyCode::Left | KeyCode::Up => Self::Previous,
            KeyCode::Right | KeyCode::Down | KeyCode::Char(' ') => Self::Next,
            KeyCode::Home => Self::First,
            KeyCode::End => Self::Last,
            KeyCode::Char('p' | 'P') => Self::Pause,
            KeyCode::Char('r' | 'R') => Self::Resume,
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
            Self::Previous => "Left",
            Self::Next => "Right",
            Self::Last => "End",
            Self::Pause => "p",
            Self::Goto(_) => "1..n",
            Self::Resume => "r",
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
            Self::Previous => ActionDetails::new(vec!["Left", "Up", "Space"], "Go to next slide"),
            Self::Next => ActionDetails::new(vec!["Right", "Down"], "Go to previous slide"),
            Self::Last => ActionDetails::new(vec!["End"], "Go to last slide"),
            Self::Pause => ActionDetails::new(vec!["p", "P"], "Pause"),
            Self::Goto(_) => ActionDetails::new(vec!["1..n"], "Go to slide n"),
            Self::Resume => ActionDetails::new(vec!["r", "R"], "Resume"),
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
            Self::Previous => Command::Previous,
            Self::Next => Command::Next,
            Self::Last => Command::Last,
            Self::Pause => Command::Pause,
            Self::Resume => Command::Resume,
            Self::Blink => Command::Blink,
            Self::Goto(id) => Command::GoTo(id.into()),
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
