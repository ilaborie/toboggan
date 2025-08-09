use std::sync::Arc;

use anyhow::Result;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, error, info, warn};
use url::Url;

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

// Internal WebSocket command format (matching server protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
enum WsCommand {
    Register { client: String },
    First,
    Last,
    GoTo(SlideId),
    Next,
    Previous,
    Pause,
    Resume,
    Ping,
}

// Internal WebSocket notification format (matching server protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsNotification {
    State { timestamp: String, state: WsState },
    Error { timestamp: String, message: String },
    Pong { timestamp: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsState {
    Running {
        current: SlideId,
        total_duration: u64,
    },
    Paused {
        current: SlideId,
        total_duration: u64,
    },
    Done {
        current: SlideId,
        total_duration: u64,
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

// WebSocket connection handles
#[derive(Debug)]
struct WebSocketConnection {
    command_sender: mpsc::UnboundedSender<WsCommand>,
    disconnect_sender: mpsc::UnboundedSender<()>,
}

// Main client implementation
#[derive(uniffi::Object)]
pub struct TobogganClient {
    config: ClientConfig,
    connection_state: Arc<RwLock<ConnectionState>>,
    current_state: Arc<RwLock<Option<State>>>,
    talk_info: Arc<RwLock<Option<TalkInfo>>>,
    ws_connection: Arc<RwLock<Option<WebSocketConnection>>>,
    client_id: String,
}

#[uniffi::export]
impl TobogganClient {
    #[uniffi::constructor]
    #[must_use]
    pub fn new(config: ClientConfig) -> Arc<Self> {
        let client_id = uuid::Uuid::new_v4().to_string();
        Arc::new(Self {
            config,
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            current_state: Arc::new(RwLock::new(None)),
            talk_info: Arc::new(RwLock::new(None)),
            ws_connection: Arc::new(RwLock::new(None)),
            client_id,
        })
    }

    /// Connect to the Toboggan server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if connection fails or times out
    pub async fn connect(&self) -> Result<(), TobogganError> {
        info!("Connecting to {}", self.config.websocket_url);

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connecting;
        }

        // Parse the WebSocket URL
        let url =
            Url::parse(&self.config.websocket_url).map_err(|err| TobogganError::ConfigError {
                message: format!("Invalid WebSocket URL: {err}"),
            })?;

        // Attempt connection with retries
        let mut retries = 0;
        let websocket_stream = loop {
            match connect_async(url.as_str()).await {
                Ok((stream, _)) => break stream,
                Err(err) => {
                    if retries >= self.config.max_retries {
                        let mut state = self.connection_state.write().await;
                        let error_msg = format!("Failed to connect after {retries} retries: {err}");
                        *state = ConnectionState::Error(error_msg.clone());
                        return Err(TobogganError::ConnectionError { message: error_msg });
                    }

                    warn!(
                        "Connection attempt {} failed: {err}, retrying...",
                        retries + 1
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(
                        self.config.retry_delay_ms,
                    ))
                    .await;
                    retries += 1;
                }
            }
        };

        info!("WebSocket connection established");

        // Split the WebSocket stream
        let (ws_sender, ws_receiver) = websocket_stream.split();

        // Create channels for communication
        let (command_tx, command_rx) = mpsc::unbounded_channel::<WsCommand>();
        let (disconnect_tx, disconnect_rx) = mpsc::unbounded_channel::<()>();

        // Store connection handles
        {
            let mut ws_conn = self.ws_connection.write().await;
            *ws_conn = Some(WebSocketConnection {
                command_sender: command_tx,
                disconnect_sender: disconnect_tx,
            });
        }

        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Connected;
        }

        // Clone Arcs for background tasks
        let state_handle = Arc::clone(&self.current_state);
        let talk_handle = Arc::clone(&self.talk_info);
        let connection_state_handle = Arc::clone(&self.connection_state);
        let client_id = self.client_id.clone();

        // Fetch initial talk info via HTTP
        if let Err(err) = self.fetch_talk_info().await {
            warn!("Failed to fetch talk info via HTTP: {err}");
            // Continue anyway, we'll get updates via WebSocket
        }

        // Spawn background tasks
        Self::spawn_websocket_tasks(
            ws_sender,
            ws_receiver,
            ClientInner {
                command_rx,
                disconnect_rx,
                state_handle,
                talk_handle,
                connection_state_handle,
                client_id,
            },
        );

        Ok(())
    }

    /// Disconnect from the Toboggan server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if disconnection fails
    pub async fn disconnect(&self) -> Result<(), TobogganError> {
        info!("Disconnecting from WebSocket");

        // Send disconnect signal to background tasks
        if let Some(connection) = self.ws_connection.read().await.as_ref() {
            let _ = connection.disconnect_sender.send(());
        }

        // Clear connection handles
        {
            let mut ws_conn = self.ws_connection.write().await;
            *ws_conn = None;
        }

        // Update state
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Disconnected;
        }

        // Clear presentation data
        {
            let mut state = self.current_state.write().await;
            *state = None;
        }

        {
            let mut talk = self.talk_info.write().await;
            *talk = None;
        }

        info!("Disconnected successfully");
        Ok(())
    }

    /// Send a command to the server
    ///
    /// # Errors
    ///
    /// Returns `TobogganError` if command sending fails
    pub async fn send_command(&self, command: &Command) -> Result<(), TobogganError> {
        debug!("Sending command: {:?}", command);

        // Check if we're connected
        if !self.is_connected().await {
            return Err(TobogganError::ConnectionError {
                message: "Not connected to server".to_string(),
            });
        }

        // Convert to WebSocket command format
        let ws_command = match *command {
            Command::Next => WsCommand::Next,
            Command::Previous => WsCommand::Previous,
            Command::First => WsCommand::First,
            Command::Last => WsCommand::Last,
            Command::Play | Command::Resume => WsCommand::Resume,
            Command::Pause => WsCommand::Pause,
        };

        // Send command through WebSocket
        if let Some(connection) = self.ws_connection.read().await.as_ref() {
            connection.command_sender.send(ws_command).map_err(|_| {
                TobogganError::ConnectionError {
                    message: "Failed to send command, connection may be closed".to_string(),
                }
            })?;

            info!("Command sent successfully: {:?}", command);
            Ok(())
        } else {
            Err(TobogganError::ConnectionError {
                message: "No active WebSocket connection".to_string(),
            })
        }
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

struct ClientInner {
    command_rx: mpsc::UnboundedReceiver<WsCommand>,
    disconnect_rx: mpsc::UnboundedReceiver<()>,
    state_handle: Arc<RwLock<Option<State>>>,
    talk_handle: Arc<RwLock<Option<TalkInfo>>>,
    connection_state_handle: Arc<RwLock<ConnectionState>>,
    client_id: String,
}

impl TobogganClient {
    /// Spawn WebSocket background tasks
    fn spawn_websocket_tasks(
        ws_sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        ws_receiver: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        client_inner: ClientInner,
    ) {
        let ClientInner {
            command_rx,
            disconnect_rx,
            state_handle,
            talk_handle,
            connection_state_handle,
            client_id,
        } = client_inner;
        // Spawn sender task
        let sender_task = tokio::spawn(websocket_sender_task(
            ws_sender,
            command_rx,
            disconnect_rx,
            client_id.clone(),
        ));

        // Spawn receiver task
        let receiver_task = tokio::spawn(websocket_receiver_task(
            ws_receiver,
            state_handle,
            talk_handle,
            connection_state_handle,
            client_id,
        ));

        // Don't wait for tasks to complete as they run in the background
        tokio::spawn(async move {
            tokio::select! {
                _ = sender_task => {
                    debug!("WebSocket sender task completed");
                }
                _ = receiver_task => {
                    debug!("WebSocket receiver task completed");
                }
            }
        });
    }
}

/// WebSocket sender task - handles outgoing commands
async fn websocket_sender_task(
    mut ws_sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
    mut command_rx: mpsc::UnboundedReceiver<WsCommand>,
    mut disconnect_rx: mpsc::UnboundedReceiver<()>,
    client_id: String,
) {
    info!("WebSocket sender task started for client {}", client_id);

    // Send initial registration
    let register_cmd = WsCommand::Register {
        client: client_id.clone(),
    };
    if let Ok(msg) = serde_json::to_string(&register_cmd) {
        if let Err(err) = ws_sender.send(WsMessage::Text(msg)).await {
            error!("Failed to send registration command: {err}");
            return;
        }
        info!("Client {} registered with server", client_id);
    }

    loop {
        tokio::select! {
            // Handle disconnect signal
            _ = disconnect_rx.recv() => {
                info!("Received disconnect signal, closing WebSocket sender");
                let _ = ws_sender.close().await;
                break;
            }

            // Handle outgoing commands
            command = command_rx.recv() => {
                if let Some(cmd) = command {
                    debug!("Sending WebSocket command: {:?}", cmd);

                    match serde_json::to_string(&cmd) {
                        Ok(msg) => {
                            if let Err(err) = ws_sender.send(WsMessage::Text(msg)).await {
                                error!("Failed to send WebSocket message: {err}");
                                break;
                            }
                        }
                        Err(err) => {
                            error!("Failed to serialize command: {err}");
                        }
                    }
                } else {
                    info!("Command channel closed, stopping sender task");
                    break;
                }
            }
        }
    }

    info!("WebSocket sender task ended for client {}", client_id);
}

/// WebSocket receiver task - handles incoming notifications
async fn websocket_receiver_task(
    mut ws_receiver: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    state_handle: Arc<RwLock<Option<State>>>,
    talk_handle: Arc<RwLock<Option<TalkInfo>>>,
    connection_state_handle: Arc<RwLock<ConnectionState>>,
    client_id: String,
) {
    info!("WebSocket receiver task started for client {}", client_id);

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                debug!("Received WebSocket message: {}", text);

                match serde_json::from_str::<WsNotification>(&text) {
                    Ok(notification) => {
                        handle_notification(notification, &state_handle, &talk_handle, &client_id)
                            .await;
                    }
                    Err(err) => {
                        warn!("Failed to parse WebSocket notification: {err}");
                    }
                }
            }
            Ok(WsMessage::Close(_)) => {
                info!("WebSocket connection closed by server");
                break;
            }
            Ok(WsMessage::Ping(_) | WsMessage::Pong(_)) => {
                debug!("Received WebSocket ping/pong");
            }
            Ok(WsMessage::Binary(_)) => {
                warn!("Received unexpected binary WebSocket message");
            }
            Ok(WsMessage::Frame(_)) => {
                debug!("Received raw WebSocket frame");
            }
            Err(err) => {
                error!("WebSocket receiver error: {err}");
                break;
            }
        }
    }

    // Update connection state when receiver task ends
    {
        let mut state = connection_state_handle.write().await;
        if matches!(*state, ConnectionState::Connected) {
            *state = ConnectionState::Disconnected;
            info!("Connection state updated to Disconnected");
        }
    }

    info!("WebSocket receiver task ended for client {}", client_id);
}

/// Handle incoming WebSocket notifications
async fn handle_notification(
    notification: WsNotification,
    state_handle: &Arc<RwLock<Option<State>>>,
    _talk_handle: &Arc<RwLock<Option<TalkInfo>>>,
    client_id: &str,
) {
    match notification {
        WsNotification::State { state, .. } => {
            debug!("Received state update: {:?}", state);

            // Convert WebSocket state to our state format
            let new_state = match state {
                WsState::Running {
                    current,
                    total_duration,
                } => State::Running {
                    current,
                    total_duration_ms: total_duration,
                },
                WsState::Paused {
                    current,
                    total_duration,
                } => State::Paused {
                    current,
                    total_duration_ms: total_duration,
                },
                WsState::Done {
                    current,
                    total_duration,
                } => State::Done {
                    current,
                    total_duration_ms: total_duration,
                },
            };

            // Update state
            {
                let mut current_state = state_handle.write().await;
                *current_state = Some(new_state);
            }

            info!("State updated for client {}", client_id);
        }
        WsNotification::Error { message, .. } => {
            error!("Received error from server: {}", message);
        }
        WsNotification::Pong { .. } => {
            debug!("Received pong from server");
        }
    }
}

impl TobogganClient {
    /// Fetch talk information via HTTP REST API
    async fn fetch_talk_info(&self) -> Result<(), TobogganError> {
        info!("Fetching talk info via HTTP");

        // Convert WebSocket URL to HTTP URL
        let base_url = self
            .config
            .websocket_url
            .replace("ws://", "http://")
            .replace("wss://", "https://")
            .replace("/api/ws", "");

        let talk_url = format!("{base_url}/api/talk");

        let client = reqwest::Client::new();
        let response =
            client
                .get(&talk_url)
                .send()
                .await
                .map_err(|err| TobogganError::ConnectionError {
                    message: format!("Failed to fetch talk info: {err}"),
                })?;

        if !response.status().is_success() {
            return Err(TobogganError::ConnectionError {
                message: format!(
                    "HTTP error {}: {}",
                    response.status(),
                    response.status().canonical_reason().unwrap_or("Unknown")
                ),
            });
        }

        let talk_response: TalkApiResponse =
            response
                .json()
                .await
                .map_err(|err| TobogganError::ParseError {
                    message: format!("Failed to parse talk response: {err}"),
                })?;

        // Convert to our TalkInfo format
        let talk_info = convert_talk_to_talk_info(talk_response.talk);

        // Update talk info
        {
            let mut talk = self.talk_info.write().await;
            *talk = Some(talk_info);
        }

        info!("Successfully fetched talk info via HTTP");
        Ok(())
    }
}

/// Convert server Talk format to our `TalkInfo` format
fn convert_talk_to_talk_info(talk: ServerTalk) -> TalkInfo {
    let slides = talk
        .slides
        .into_iter()
        .map(|slide| Slide {
            id: slide.id.to_string(),
            title: slide.title.map(content_to_string).unwrap_or_default(),
            body: content_to_string(slide.body),
            kind: match slide.kind {
                ServerSlideKind::Cover => SlideKind::Cover,
                ServerSlideKind::Part => SlideKind::Part,
                ServerSlideKind::Standard => SlideKind::Standard,
            },
            style: match slide.style {
                ServerSlideStyle::Default => SlideStyle::Default,
                ServerSlideStyle::Center => SlideStyle::Center,
                ServerSlideStyle::Code => SlideStyle::Code,
            },
            notes: slide.notes.map(content_to_string),
        })
        .collect();

    TalkInfo {
        title: content_to_string(talk.title),
        date: talk.date.map(|date| date.to_string()),
        slides,
    }
}

/// Convert server Content to string for display
fn content_to_string(content: ServerContent) -> String {
    match content {
        ServerContent::Text(text) => text,
        ServerContent::Html { html, alt } => alt.unwrap_or(html),
        ServerContent::Md { md, alt } => alt.unwrap_or(md),
        ServerContent::IFrame { url } => format!("IFrame: {url}"),
        ServerContent::Term { .. } => "Terminal".to_string(),
        ServerContent::HBox { contents, .. } => contents
            .into_iter()
            .map(content_to_string)
            .collect::<Vec<_>>()
            .join(" | "),
        ServerContent::VBox { contents, .. } => contents
            .into_iter()
            .map(content_to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

// HTTP API response types (matching server format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TalkApiResponse {
    talk: ServerTalk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerTalk {
    title: ServerContent,
    date: Option<ServerDate>,
    slides: Vec<ServerSlide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerSlide {
    id: ServerSlideId,
    kind: ServerSlideKind,
    style: ServerSlideStyle,
    title: Option<ServerContent>,
    body: ServerContent,
    notes: Option<ServerContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ServerContent {
    Text(String),
    Html {
        html: String,
        alt: Option<String>,
    },
    Md {
        md: String,
        alt: Option<String>,
    },
    IFrame {
        url: String,
    },
    Term {
        dir: String,
        bootstrap: Vec<String>,
    },
    HBox {
        sizes: String,
        contents: Vec<ServerContent>,
    },
    VBox {
        sizes: String,
        contents: Vec<ServerContent>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerSlideKind {
    Cover,
    Part,
    Standard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerSlideStyle {
    Default,
    Center,
    Code,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerSlideId(u32);

impl std::fmt::Display for ServerSlideId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerDate {
    year: u16,
    month: u8,
    day: u8,
}

impl std::fmt::Display for ServerDate {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
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
