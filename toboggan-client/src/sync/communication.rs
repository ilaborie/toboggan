use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use toboggan_core::{ClientId, Command, Notification, Renderer, State};
use tracing::{debug, error, info, warn};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket, connect};

use crate::TobogganWebsocketConfig;

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting {
        attempt: usize,
        max_attempt: usize,
        delay: Duration,
    },
    Error {
        message: String,
    },
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(f, "📡 Connecting..."),
            Self::Connected => write!(f, "🛜 Connected"),
            Self::Closed => write!(f, "🚪 Closed"),
            Self::Reconnecting {
                delay,
                attempt,
                max_attempt,
            } => {
                write!(
                    f,
                    "⛓️‍💥 Reconnecting in {}s {attempt}/{max_attempt}",
                    delay.as_secs()
                )
            }
            Self::Error { message } => write!(f, "💥 Error: {message}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommunicationMessage {
    ConnectionStatusChange { status: ConnectionStatus },
    StateChange { state: State },
    Error { error: String },
}

/// Trait for receiving notifications from the WebSocket client
pub trait NotificationCallback: Send + Sync + 'static {
    fn on_notification(&self, message: CommunicationMessage);
}

impl<F> NotificationCallback for F
where
    F: Fn(CommunicationMessage) + Send + Sync + 'static,
{
    fn on_notification(&self, message: CommunicationMessage) {
        self(message);
    }
}

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

pub struct WebSocketClient {
    client_id: ClientId,
    config: TobogganWebsocketConfig,
    callback: Arc<dyn NotificationCallback>,
    cmd_sender: Option<Sender<Command>>,
    state: Arc<Mutex<ConnectionState>>,
    worker_handle: Option<JoinHandle<()>>,
    ping_shutdown: Option<Sender<()>>,
    shutdown_sender: Option<Sender<()>>,
}

impl WebSocketClient {
    #[must_use]
    pub fn new(
        client_id: ClientId,
        config: TobogganWebsocketConfig,
        callback: Arc<dyn NotificationCallback>,
    ) -> Self {
        Self {
            client_id,
            config,
            callback,
            cmd_sender: None,
            state: Arc::new(Mutex::new(ConnectionState::default())),
            worker_handle: None,
            ping_shutdown: None,
            shutdown_sender: None,
        }
    }

    fn send_status_change(&self, status: ConnectionStatus) {
        debug!(%status, "🗿connection status");
        let message = CommunicationMessage::ConnectionStatusChange { status };
        self.callback.on_notification(message);
    }

    pub fn connect(&mut self) {
        let state = self.state.lock().expect("Failed to lock state");
        if state.is_disposed {
            warn!("Illegal disposed state, cannot connect");
            return;
        }
        drop(state);

        self.attempt_connection();
    }

    fn attempt_connection(&mut self) {
        // Notify connecting status
        self.send_status_change(ConnectionStatus::Connecting);

        // Create command channel
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

        self.cmd_sender = Some(cmd_tx);
        self.shutdown_sender = Some(shutdown_tx);

        // Spawn worker thread for WebSocket communication
        let config = self.config.clone();
        let client_id = self.client_id;
        let callback = Arc::clone(&self.callback);
        let state = Arc::clone(&self.state);

        let worker_handle = thread::spawn(move || {
            Self::worker_thread(config, client_id, callback, state, cmd_rx, shutdown_rx);
        });

        self.worker_handle = Some(worker_handle);
    }

    fn worker_thread(
        config: TobogganWebsocketConfig,
        client_id: ClientId,
        callback: Arc<dyn NotificationCallback>,
        state: Arc<Mutex<ConnectionState>>,
        cmd_rx: Receiver<Command>,
        shutdown_rx: Receiver<()>,
    ) {
        loop {
            // Check if we should stop
            if let Ok(_) = shutdown_rx.try_recv() {
                debug!("Worker thread received shutdown signal");
                break;
            }

            // Check if disposed
            {
                let state_guard = state.lock().expect("Failed to lock state");
                if state_guard.is_disposed {
                    break;
                }
            }

            // Attempt connection
            let (mut ws, _) = match connect(&config.websocket_url) {
                Ok(ws) => ws,
                Err(error) => {
                    error!(?error, "Failed to open WebSocket");
                    let message = CommunicationMessage::ConnectionStatusChange {
                        status: ConnectionStatus::Error {
                            message: error.to_string(),
                        },
                    };
                    callback.on_notification(message);
                    Self::schedule_reconnect(&config, &state, &callback);
                    continue;
                }
            };

            // Reset retry state and notify connected
            {
                let mut state_guard = state.lock().expect("Failed to lock state");
                state_guard.retry_count = 0;
                state_guard.retry_delay = config.retry_delay;
            }

            let message = CommunicationMessage::ConnectionStatusChange {
                status: ConnectionStatus::Connected,
            };
            callback.on_notification(message);

            // Register with server
            if let Err(error) = Self::send_register_command(&mut ws, client_id) {
                error!(?error, "Failed to register with server");
                continue;
            }

            // Handle communication
            let result = Self::handle_websocket_communication(ws, &cmd_rx, &shutdown_rx, &callback);

            if let Err(error) = result {
                error!(?error, "WebSocket communication error");
                let message = CommunicationMessage::Error {
                    error: error.to_string(),
                };
                callback.on_notification(message);
            }

            // Connection closed, check if we should reconnect
            let should_reconnect = {
                let state_guard = state.lock().expect("Failed to lock state");
                !state_guard.is_disposed
            };

            if should_reconnect {
                warn!("⚠️ WebSocket connection closed, will attempt reconnection");
                let message = CommunicationMessage::ConnectionStatusChange {
                    status: ConnectionStatus::Closed,
                };
                callback.on_notification(message);
                Self::schedule_reconnect(&config, &state, &callback);
            } else {
                break;
            }
        }
    }

    fn send_register_command(
        ws: &mut WebSocket<MaybeTlsStream<TcpStream>>,
        client_id: ClientId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let register_cmd = Command::Register {
            client: client_id,
            renderer: Renderer::default(),
        };
        let json = serde_json::to_string(&register_cmd)?;
        ws.send(Message::text(json))?;
        Ok(())
    }

    fn handle_websocket_communication(
        ws: WebSocket<MaybeTlsStream<TcpStream>>,
        cmd_rx: &Receiver<Command>,
        shutdown_rx: &Receiver<()>,
        callback: &Arc<dyn NotificationCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Split the WebSocket into separate read and write halves
        // This allows concurrent reading and writing without blocking
        let ws = Arc::new(Mutex::new(ws));
        let ws_write = Arc::clone(&ws);
        let ws_read = Arc::clone(&ws);

        // Create channels for thread communication
        let (write_stop_tx, write_stop_rx) = mpsc::channel::<()>();
        let (cmd_forward_tx, cmd_forward_rx) = mpsc::channel::<Command>();

        // Spawn a separate thread for writing (commands and pings)
        let write_handle = thread::spawn(move || {
            let mut last_ping = Instant::now();
            loop {
                // Check for shutdown
                if let Ok(_) = write_stop_rx.try_recv() {
                    debug!("Write thread received stop signal");
                    break;
                }

                // Send ping if needed
                if last_ping.elapsed() >= PING_PERIOD {
                    let ping_cmd = Command::Ping;
                    if let Ok(json) = serde_json::to_string(&ping_cmd) {
                        if let Ok(mut ws) = ws_write.lock() {
                            if let Err(e) = ws.send(Message::text(json)) {
                                error!("Failed to send ping: {}", e);
                                break;
                            }
                            debug!("Sent ping");
                        }
                    }
                    last_ping = Instant::now();
                }

                // Check for outgoing commands (blocking wait with timeout)
                // This is the key fix - we use recv_timeout instead of try_recv
                // so the write thread actually waits for commands
                match cmd_forward_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(cmd) => {
                        println!("🦀 WebSocket: Sending command {:?}", cmd);
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            if let Ok(mut ws) = ws_write.lock() {
                                if let Err(e) = ws.send(Message::text(json)) {
                                    error!("Failed to send command: {}", e);
                                    break;
                                }
                                info!("Command sent via WebSocket: {:?}", cmd);
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Normal timeout, continue loop
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        debug!("Command channel disconnected");
                        break;
                    }
                }
            }
            debug!("Write thread exiting");
        });

        // Read thread (main thread continues as read thread)
        loop {
            // Check for shutdown signal
            if let Ok(_) = shutdown_rx.try_recv() {
                debug!("Read thread received shutdown signal");
                let _ = write_stop_tx.send(());
                break;
            }

            // Forward any commands from the main channel to the write thread
            if let Ok(cmd) = cmd_rx.try_recv() {
                let _ = cmd_forward_tx.send(cmd);
            }

            // Read messages (with timeout to allow checking for commands)
            let message = {
                let mut ws = ws_read.lock().unwrap();
                ws.read()
            };

            match message {
                Ok(Message::Text(text)) => {
                    Self::handle_incoming_message(&text, callback);
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket close message received");
                    let _ = write_stop_tx.send(());
                    break;
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(tungstenite::Error::Io(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Timeout is expected, continue to check for commands
                    continue;
                }
                Err(error) => {
                    error!(?error, "WebSocket read error");
                    let _ = write_stop_tx.send(());
                    return Err(Box::new(error));
                }
            }
        }

        // Wait for write thread to finish
        let _ = write_handle.join();

        Ok(())
    }

    fn handle_incoming_message(text: &str, callback: &Arc<dyn NotificationCallback>) {
        let notification = match serde_json::from_str::<Notification>(text) {
            Ok(notification) => notification,
            Err(error) => {
                error!(?error, ?text, "Failed to deserialize notification");
                return;
            }
        };

        match notification {
            Notification::State { state, .. } => {
                let message = CommunicationMessage::StateChange { state };
                callback.on_notification(message);
            }
            Notification::Error { message, .. } => {
                let msg = CommunicationMessage::Error { error: message };
                callback.on_notification(msg);
            }
            Notification::Pong { .. } => {
                debug!("⏱️ Pong received");
            }
            Notification::Blink => {
                info!("🔔 Blink");
            }
        }
    }

    fn schedule_reconnect(
        config: &TobogganWebsocketConfig,
        state: &Arc<Mutex<ConnectionState>>,
        callback: &Arc<dyn NotificationCallback>,
    ) {
        let (retry_count, retry_delay, should_reconnect) = {
            let mut state_guard = state.lock().expect("Failed to lock state");
            if state_guard.is_disposed || state_guard.retry_count >= config.max_retries {
                return;
            }

            state_guard.retry_count += 1;
            let fixed_delay = Duration::from_secs(5);

            (state_guard.retry_count, fixed_delay, true)
        };

        if should_reconnect {
            let status = ConnectionStatus::Reconnecting {
                attempt: retry_count,
                max_attempt: config.max_retries,
                delay: retry_delay,
            };
            let message = CommunicationMessage::ConnectionStatusChange { status };
            callback.on_notification(message);

            // Sleep for the retry delay
            thread::sleep(retry_delay);
        }
    }

    pub fn send_command(&self, command: Command) -> Result<(), String> {
        if let Some(sender) = &self.cmd_sender {
            sender
                .send(command)
                .map_err(|e| format!("Failed to send command: {e}"))
        } else {
            Err("Not connected".to_string())
        }
    }

    pub fn disconnect(&mut self) {
        // Mark as disposed
        {
            let mut state = self.state.lock().expect("Failed to lock state");
            state.is_disposed = true;
        }

        // Send shutdown signal
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.send(());
        }

        // Wait for worker thread to finish
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        // Signal ping thread to shutdown
        if let Some(ping_shutdown) = self.ping_shutdown.take() {
            let _ = ping_shutdown.send(());
        }

        self.cmd_sender = None;
    }
}
