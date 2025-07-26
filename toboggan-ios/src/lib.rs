use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

// Re-export uniffi macros - but simplified without UDL
uniffi::setup_scaffolding!();

// Custom error type for UniFFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum TobogganError {
    #[error("Connection error: {message}")]
    ConnectionError { message: String },
    #[error("Parse error: {message}")]
    ParseError { message: String },
    #[error("Config error: {message}")]
    ConfigError { message: String },
    #[error("Unknown error: {message}")]
    UnknownError { message: String },
}

impl From<anyhow::Error> for TobogganError {
    fn from(err: anyhow::Error) -> Self {
        Self::UnknownError {
            message: err.to_string(),
        }
    }
}

// Configuration for the client
#[derive(Debug, Clone, uniffi::Record)]
pub struct ClientConfig {
    pub websocket_url: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

// UniFFI-compatible types
pub type SlideId = String;

#[derive(Debug, Clone, uniffi::Enum)]
pub enum SlideKind {
    Cover,
    Part,
    Standard,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum SlideStyle {
    Default,
    Center,
    Code,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Slide {
    pub id: SlideId,
    pub title: String,
    pub body: String,
    pub kind: SlideKind,
    pub style: SlideStyle,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct TalkInfo {
    pub title: String,
    pub date: Option<String>,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Command {
    Next,
    Previous,
    First,
    Last,
    Play,
    Pause,
    Resume,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum State {
    Running {
        current: SlideId,
        total_duration_ms: u64,
    },
    Paused {
        current: SlideId,
        total_duration_ms: u64,
    },
    Done {
        current: SlideId,
        total_duration_ms: u64,
    },
}

// Connection state
#[derive(Debug)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    #[allow(dead_code)]
    Error(String),
}

// Main client implementation
#[derive(uniffi::Object)]
pub struct TobogganClient {
    config: ClientConfig,
    connection_state: Arc<RwLock<ConnectionState>>,
    current_state: Arc<RwLock<Option<State>>>,
    talk_info: Arc<RwLock<Option<TalkInfo>>>,
}

#[uniffi::export]
impl TobogganClient {
    #[uniffi::constructor]
    #[must_use]
    pub fn new(config: ClientConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            current_state: Arc::new(RwLock::new(None)),
            talk_info: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to the Toboggan server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if connection fails or times out
    pub async fn connect(&self) -> Result<(), TobogganError> {
        tracing::info!("Connecting to {}", self.config.websocket_url);

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connecting;
        }

        // For now, just simulate a successful connection
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connected;
        }

        // Set up some mock data
        let mock_talk = TalkInfo {
            title: "Demo Presentation".to_string(),
            date: Some("2025-01-26".to_string()),
            slides: vec![
                Slide {
                    id: "1".to_string(),
                    title: "Welcome".to_string(),
                    body: "Welcome to Toboggan".to_string(),
                    kind: SlideKind::Cover,
                    style: SlideStyle::Default,
                    notes: Some("This is the opening slide".to_string()),
                },
                Slide {
                    id: "2".to_string(),
                    title: "Introduction".to_string(),
                    body: "Let's get started".to_string(),
                    kind: SlideKind::Standard,
                    style: SlideStyle::Default,
                    notes: None,
                },
            ],
        };

        {
            let mut talk = self.talk_info.write().await;
            *talk = Some(mock_talk);
        }

        {
            let mut state = self.current_state.write().await;
            *state = Some(State::Paused {
                current: "1".to_string(),
                total_duration_ms: 0,
            });
        }

        Ok(())
    }

    /// Disconnect from the Toboggan server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if disconnection fails
    pub async fn disconnect(&self) -> Result<(), TobogganError> {
        tracing::info!("Disconnecting");

        // Update state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Disconnected;
        }

        Ok(())
    }

    /// Send a command to the server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if command sending fails
    pub fn send_command(&self, command: &Command) -> Result<(), TobogganError> {
        tracing::debug!("Sending command: {:?}", command);

        // For now, just log the command
        match *command {
            Command::Next => tracing::info!("Next slide requested"),
            Command::Previous => tracing::info!("Previous slide requested"),
            Command::First => tracing::info!("First slide requested"),
            Command::Last => tracing::info!("Last slide requested"),
            Command::Play | Command::Resume => tracing::info!("Resume requested"),
            Command::Pause => tracing::info!("Pause requested"),
        }

        Ok(())
    }

    pub async fn get_current_state(&self) -> Option<State> {
        let state = self.current_state.read().await;
        state.clone()
    }

    pub async fn get_slide(&self, slide_id: SlideId) -> Option<Slide> {
        let talk = self.talk_info.read().await;
        if let Some(talk_info) = talk.as_ref() {
            talk_info
                .slides
                .iter()
                .find(|slide| slide.id == slide_id)
                .cloned()
        } else {
            None
        }
    }

    pub async fn get_talk_info(&self) -> Option<TalkInfo> {
        let talk = self.talk_info.read().await;
        talk.clone()
    }

    pub async fn is_connected(&self) -> bool {
        let state = self.connection_state.read().await;
        matches!(*state, ConnectionState::Connected)
    }
}

/// Create a new Toboggan client
///
/// # Errors
///
/// Returns `TobogganError` if client creation fails
#[uniffi::export]
pub fn create_client(config: ClientConfig) -> Result<Arc<TobogganClient>, TobogganError> {
    Ok(TobogganClient::new(config))
}
