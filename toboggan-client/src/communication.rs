use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use toboggan_core::{ClientId, Command, Notification, Renderer, State};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, error, info, warn};

use crate::TobogganWebsocketConfig;

type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, derive_more::Display)]
pub enum ConnectionStatus {
    #[display("📡 Connecting...")]
    Connecting,
    #[display("🛜 Connected")]
    Connected,
    #[display("🚪 Closed")]
    Closed,
    #[display("⛓️‍💥 Reconnecting in {}s {attempt}/{max_attempt}", delay.as_secs())]
    Reconnecting {
        attempt: usize,
        max_attempt: usize,
        delay: Duration,
    },
    #[display("💥 Error: {message}")]
    Error { message: String },
}

#[derive(Debug, Clone)]
pub enum CommunicationMessage {
    ConnectionStatusChange { status: ConnectionStatus },
    StateChange { state: State },
    Error { error: String },
}

#[derive(Clone)]
struct ConnectionState {
    retry_count: usize,
    retry_delay: Duration,
    is_disposed: bool,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            retry_count: 0,
            retry_delay: Duration::from_secs(1),
            is_disposed: false,
        }
    }
}

const PING_PERIOD: Duration = Duration::from_secs(10);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub struct WebSocketClient {
    client_id: ClientId,
    config: TobogganWebsocketConfig,
    tx_msg: mpsc::UnboundedSender<CommunicationMessage>,
    tx_cmd: mpsc::UnboundedSender<Command>,
    rx_cmd: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    state: Arc<Mutex<ConnectionState>>,
    ping_task: Option<JoinHandle<()>>,
    last_ping: Arc<Mutex<Option<Instant>>>,
}

fn create_reconnecting_status(
    attempt: usize,
    max_attempt: usize,
    delay: Duration,
) -> ConnectionStatus {
    ConnectionStatus::Reconnecting {
        attempt,
        max_attempt,
        delay,
    }
}

impl WebSocketClient {
    #[must_use]
    pub fn new(
        tx_cmd: mpsc::UnboundedSender<Command>,
        rx_cmd: mpsc::UnboundedReceiver<Command>,
        client_id: ClientId,
        config: TobogganWebsocketConfig,
    ) -> (Self, mpsc::UnboundedReceiver<CommunicationMessage>) {
        let (tx_msg, rx_msg) = mpsc::unbounded_channel();

        let state = ConnectionState::default();
        let state = Arc::new(Mutex::new(state));

        let rx_cmd = Arc::new(Mutex::new(rx_cmd));

        let result = Self {
            client_id,
            config,
            tx_msg,
            tx_cmd,
            rx_cmd,
            state,
            ping_task: None,
            last_ping: Arc::default(),
        };
        (result, rx_msg)
    }

    fn send_status_change(&self, status: ConnectionStatus) {
        debug!(%status, "🗿connection status");
        let _ = self
            .tx_msg
            .send(CommunicationMessage::ConnectionStatusChange { status });
    }

    pub async fn connect(&mut self) {
        let state = self.state.lock().await;
        if state.is_disposed {
            warn!("Illegal disposed state, cannot connect");
            return;
        }
        mem::drop(state);

        self.attempt_connection().await;
    }

    async fn attempt_connection(&mut self) {
        // Notify connecting status
        self.send_status_change(ConnectionStatus::Connecting);

        let (ws, _) = match connect_async(&self.config.websocket_url).await {
            Ok(ws) => ws,
            Err(error) => {
                error!(?error, "Failed to open WebSocket");
                self.send_status_change(ConnectionStatus::Error {
                    message: error.to_string(),
                });
                self.schedule_reconnect().await;
                return;
            }
        };

        let (write, read) = ws.split();

        // Handle connection success
        self.handle_connection_open().await;

        // Handle outgoing messages
        let rx_cmd = Arc::clone(&self.rx_cmd);
        tokio::spawn(handle_outgoing_commands(rx_cmd, write));

        // Handle incoming messages and connection lifecycle
        let tx_msg_clone = self.tx_msg.clone();
        let state_clone = self.state.clone();
        let config = self.config.clone();
        let last_ping = Arc::clone(&self.last_ping);
        tokio::spawn(handle_incoming_messages(
            read,
            tx_msg_clone,
            state_clone,
            config,
            last_ping,
        ));
    }

    async fn handle_connection_open(&mut self) {
        // Reset retry state
        {
            let mut state = self.state.lock().await;
            state.retry_count = 0;
            state.retry_delay = self.config.retry_delay;
        }

        // Start pinging
        self.start_pinging();

        // Notify connected status
        self.send_status_change(ConnectionStatus::Connected);
    }

    async fn schedule_reconnect(&mut self) {
        let (retry_count, retry_delay, max_retries) = {
            let mut state = self.state.lock().await;
            if state.is_disposed {
                return;
            }

            if state.retry_count >= self.config.max_retries {
                let message = format!("Max retries reached! ({})", self.config.max_retries);
                self.send_status_change(ConnectionStatus::Error { message });
                return;
            }

            state.retry_count += 1;
            (state.retry_count, RECONNECT_DELAY, self.config.max_retries)
        };

        self.send_status_change(create_reconnecting_status(
            retry_count,
            max_retries,
            retry_delay,
        ));

        // Schedule reconnection with current delay
        let tx_msg_clone = self.tx_msg.clone();
        let config = self.config.clone();
        let client_id = self.client_id;
        let state_clone = Arc::clone(&self.state);
        let tx_cmd = self.tx_cmd.clone();
        let rx_cmd = Arc::clone(&self.rx_cmd);
        let last_ping = Arc::clone(&self.last_ping);

        tokio::spawn(async move {
            tokio::time::sleep(retry_delay).await;
            let state = state_clone.lock().await;
            if !state.is_disposed {
                mem::drop(state);
                reconnect_with_channel(
                    &config,
                    client_id,
                    tx_msg_clone,
                    &tx_cmd,
                    rx_cmd,
                    last_ping,
                )
                .await;
            }
        });
    }

    fn start_pinging(&mut self) {
        // Stop any existing ping task
        if let Some(task) = self.ping_task.take() {
            task.abort();
        }

        let tx_cmd = self.tx_cmd.clone();
        let last_ping = Arc::clone(&self.last_ping);
        let mut interval = tokio::time::interval(PING_PERIOD);
        let task = tokio::spawn(async move {
            loop {
                interval.tick().await;
                let mut last_ping_guard = last_ping.lock().await;
                *last_ping_guard = Some(Instant::now());
                mem::drop(last_ping_guard);
                let _ = tx_cmd.send(Command::Ping);
            }
        });
        self.ping_task = Some(task);
    }
}

impl Drop for WebSocketClient {
    fn drop(&mut self) {
        // Mark as disposed to prevent reconnection attempts
        if let Ok(mut state) = self.state.try_lock() {
            state.is_disposed = true;
        }

        // Clear last ping state
        if let Ok(mut last_ping) = self.last_ping.try_lock() {
            last_ping.take();
        }

        // Abort ping task if running
        if let Some(task) = self.ping_task.take() {
            task.abort();
        }
    }
}

async fn reconnect_with_channel(
    config: &TobogganWebsocketConfig,
    client_id: ClientId,
    tx_msg: mpsc::UnboundedSender<CommunicationMessage>,
    tx_cmd: &mpsc::UnboundedSender<Command>,
    rx_cmd: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    last_ping: Arc<Mutex<Option<Instant>>>,
) {
    info!("Attempting to reconnect...");

    let (ws, _) = match connect_async(&config.websocket_url).await {
        Ok(ws) => ws,
        Err(error) => {
            error!(?error, "Reconnection failed");
            return;
        }
    };

    let (write, read) = ws.split();

    // Notify connected and register with server
    debug!("🗿connection status: {}", ConnectionStatus::Connected);
    let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange {
        status: ConnectionStatus::Connected,
    });
    let _ = tx_cmd.send(Command::Register {
        client: client_id,
        renderer: Renderer::default(),
    });

    // Handle outgoing and incoming messages
    tokio::spawn(handle_outgoing_commands(rx_cmd, write));
    tokio::spawn(async move {
        let mut read = read;
        while let Some(msg) = read.next().await {
            if let Ok(msg) = msg {
                handle_ws_message(msg, &tx_msg, last_ping.clone()).await;
            }
        }
    });
}

async fn handle_outgoing_commands(
    rx_cmd: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    mut write: SplitSink<WebSocket, Message>,
) {
    loop {
        let cmd = {
            let mut rx_cmd = rx_cmd.lock().await;
            rx_cmd.recv().await
        };
        let Some(cmd) = cmd else {
            break;
        };
        let json = match serde_json::to_string(&cmd) {
            Ok(json) => json,
            Err(error) => {
                error!(?error, ?cmd, "Failed to serialize command");
                continue;
            }
        };
        let item = Message::text(json);

        if let Err(error) = write.send(item).await {
            error!(?error, "Failed to send WS command");
            break;
        }
    }
}

async fn handle_incoming_messages(
    mut read: SplitStream<WebSocket>,
    tx_msg: mpsc::UnboundedSender<CommunicationMessage>,
    state: Arc<Mutex<ConnectionState>>,
    config: TobogganWebsocketConfig,
    last_ping: Arc<Mutex<Option<Instant>>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                handle_ws_message(msg, &tx_msg, last_ping.clone()).await;
            }
            Err(error) => {
                error!(?error, "Failed to read WS incoming message");
                let message = error.to_string();
                let status = ConnectionStatus::Error { message };
                debug!(%status, "🗿connection status");
                let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange { status });
                break;
            }
        }
    }

    // Connection closed - notify
    warn!("⚠️ WebSocket connection closed, will attempt reconnection in 5 seconds");
    debug!("🗿connection status: {}", ConnectionStatus::Closed);
    let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange {
        status: ConnectionStatus::Closed,
    });

    // Schedule reconnect if not disposed (fixed 5-second delay)
    let (retry_count, retry_delay, should_reconnect) = {
        let mut state_ref = state.lock().await;
        if state_ref.is_disposed || state_ref.retry_count >= config.max_retries {
            return;
        }

        state_ref.retry_count += 1;
        (state_ref.retry_count, RECONNECT_DELAY, true)
    };

    if should_reconnect {
        let status = create_reconnecting_status(retry_count, config.max_retries, retry_delay);
        debug!(%status, "🗿connection status");
        let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange { status });
    }
}

async fn handle_ws_message(
    message: Message,
    tx: &mpsc::UnboundedSender<CommunicationMessage>,
    last_ping: Arc<Mutex<Option<Instant>>>,
) {
    let Message::Text(message_text) = message else {
        error!(?message, "unexpected message kind");
        return;
    };

    let notification = match serde_json::from_str::<Notification>(&message_text) {
        Ok(notification) => notification,
        Err(error) => {
            error!(?error, ?message_text, "Failed to deserialize notification");
            return;
        }
    };

    match notification {
        Notification::State { state, .. } => {
            let _ = tx.send(CommunicationMessage::StateChange { state });
        }
        Notification::Error { message, .. } => {
            let _ = tx.send(CommunicationMessage::Error { error: message });
        }
        Notification::Pong { .. } => {
            let mut lock = last_ping.lock().await;
            if let Some(instant) = lock.take() {
                let elapsed = instant.elapsed();
                debug!(?elapsed, "⏱️ Ping");
            }
        }
        Notification::Blink => {
            info!("🔔 Blink");
        }
    }
}
