use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use toboggan_core::timeouts::PING_PERIOD;
use toboggan_core::{ClientId, ClientRole, Command, Notification, State};
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
    ConnectionStatusChange {
        status: ConnectionStatus,
    },
    StateChange {
        state: State,
    },
    TalkChange {
        state: State,
    },
    Registered {
        client_id: ClientId,
        role: ClientRole,
    },
    ClientConnected {
        client_id: ClientId,
        name: String,
    },
    ClientDisconnected {
        client_id: ClientId,
        name: String,
    },
    Error {
        error: String,
    },
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

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub struct WebSocketClient {
    client_name: String,
    config: TobogganWebsocketConfig,
    tx_msg: mpsc::UnboundedSender<CommunicationMessage>,
    tx_cmd: mpsc::UnboundedSender<Command>,
    rx_cmd: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    state: Arc<Mutex<ConnectionState>>,
    ping_task: Option<JoinHandle<()>>,
    last_ping: Arc<Mutex<Option<Instant>>>,
    client_id: Arc<RwLock<Option<ClientId>>>,
}

impl WebSocketClient {
    #[must_use]
    pub fn new(
        tx_cmd: mpsc::UnboundedSender<Command>,
        rx_cmd: mpsc::UnboundedReceiver<Command>,
        client_name: impl Into<String>,
        config: TobogganWebsocketConfig,
    ) -> (Self, mpsc::UnboundedReceiver<CommunicationMessage>) {
        let (tx_msg, rx_msg) = mpsc::unbounded_channel();

        let state = ConnectionState::default();
        let state = Arc::new(Mutex::new(state));

        let rx_cmd = Arc::new(Mutex::new(rx_cmd));

        let result = Self {
            client_name: client_name.into(),
            config,
            tx_msg,
            tx_cmd,
            rx_cmd,
            state,
            ping_task: None,
            last_ping: Arc::default(),
            client_id: Arc::default(),
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
        drop(state);

        self.attempt_connection().await;
    }

    async fn attempt_connection(&mut self) {
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

        self.handle_connection_open().await;

        let rx_cmd = Arc::clone(&self.rx_cmd);
        tokio::spawn(handle_outgoing_commands(rx_cmd, write));

        let tx_msg_clone = self.tx_msg.clone();
        let state_clone = self.state.clone();
        let config = self.config.clone();
        let last_ping = Arc::clone(&self.last_ping);
        let client_id = Arc::clone(&self.client_id);
        tokio::spawn(handle_incoming_messages(
            read,
            tx_msg_clone,
            state_clone,
            config,
            last_ping,
            client_id,
        ));
    }

    async fn handle_connection_open(&mut self) {
        {
            let mut state = self.state.lock().await;
            state.retry_count = 0;
            state.retry_delay = self.config.retry_delay;
        }

        // Send Register command with client name
        let _ = self.tx_cmd.send(Command::Register {
            name: self.client_name.clone(),
            token: self.config.presenter_token.clone(),
        });

        self.start_pinging();

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

        self.send_status_change(ConnectionStatus::Reconnecting {
            attempt: retry_count,
            max_attempt: max_retries,
            delay: retry_delay,
        });

        let tx_msg_clone = self.tx_msg.clone();
        let config = self.config.clone();
        let client_name = self.client_name.clone();
        let state_clone = Arc::clone(&self.state);
        let tx_cmd = self.tx_cmd.clone();
        let rx_cmd = Arc::clone(&self.rx_cmd);
        let last_ping = Arc::clone(&self.last_ping);
        let client_id = Arc::clone(&self.client_id);

        tokio::spawn(async move {
            tokio::time::sleep(retry_delay).await;
            let state = state_clone.lock().await;
            if !state.is_disposed {
                drop(state);
                reconnect_with_channel(
                    &config,
                    &client_name,
                    tx_msg_clone,
                    &tx_cmd,
                    rx_cmd,
                    last_ping,
                    client_id,
                )
                .await;
            }
        });
    }

    fn start_pinging(&mut self) {
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
                drop(last_ping_guard);
                let _ = tx_cmd.send(Command::Ping);
            }
        });
        self.ping_task = Some(task);
    }
}

impl Drop for WebSocketClient {
    fn drop(&mut self) {
        // Send Unregister command if we have a client_id
        if let Ok(guard) = self.client_id.try_read()
            && let Some(id) = *guard
        {
            let _ = self.tx_cmd.send(Command::Unregister { client: id });
        }

        if let Ok(mut state) = self.state.try_lock() {
            state.is_disposed = true;
        }

        if let Ok(mut last_ping) = self.last_ping.try_lock() {
            last_ping.take();
        }

        if let Some(task) = self.ping_task.take() {
            task.abort();
        }
    }
}

async fn reconnect_with_channel(
    config: &TobogganWebsocketConfig,
    client_name: &str,
    tx_msg: mpsc::UnboundedSender<CommunicationMessage>,
    tx_cmd: &mpsc::UnboundedSender<Command>,
    rx_cmd: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    last_ping: Arc<Mutex<Option<Instant>>>,
    client_id: Arc<RwLock<Option<ClientId>>>,
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

    debug!("🗿connection status: {}", ConnectionStatus::Connected);
    let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange {
        status: ConnectionStatus::Connected,
    });
    let _ = tx_cmd.send(Command::Register {
        name: client_name.to_owned(),
        token: config.presenter_token.clone(),
    });

    tokio::spawn(handle_outgoing_commands(rx_cmd, write));
    tokio::spawn(async move {
        let mut read = read;
        while let Some(msg) = read.next().await {
            if let Ok(msg) = msg {
                handle_ws_message(msg, &tx_msg, last_ping.clone(), client_id.clone()).await;
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
    client_id: Arc<RwLock<Option<ClientId>>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                handle_ws_message(msg, &tx_msg, last_ping.clone(), client_id.clone()).await;
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

    warn!("⚠️ WebSocket connection closed, will attempt reconnection in 5 seconds");
    debug!("🗿connection status: {}", ConnectionStatus::Closed);
    let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange {
        status: ConnectionStatus::Closed,
    });

    let (retry_count, retry_delay, should_reconnect) = {
        let mut state_ref = state.lock().await;
        if state_ref.is_disposed || state_ref.retry_count >= config.max_retries {
            return;
        }

        state_ref.retry_count += 1;
        (state_ref.retry_count, RECONNECT_DELAY, true)
    };

    if should_reconnect {
        let status = ConnectionStatus::Reconnecting {
            attempt: retry_count,
            max_attempt: config.max_retries,
            delay: retry_delay,
        };
        debug!(%status, "🗿connection status");
        let _ = tx_msg.send(CommunicationMessage::ConnectionStatusChange { status });
    }
}

async fn handle_ws_message(
    message: Message,
    tx: &mpsc::UnboundedSender<CommunicationMessage>,
    last_ping: Arc<Mutex<Option<Instant>>>,
    client_id: Arc<RwLock<Option<ClientId>>>,
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
        Notification::State { state } => {
            let _ = tx.send(CommunicationMessage::StateChange { state });
        }
        Notification::TalkChange { state } => {
            info!("📝 Talk changed, clients should refetch Talk and Slides");
            let _ = tx.send(CommunicationMessage::TalkChange { state });
        }
        Notification::Error { message } => {
            let _ = tx.send(CommunicationMessage::Error { error: message });
        }
        Notification::Pong => {
            let mut lock = last_ping.lock().await;
            if let Some(instant) = lock.take() {
                let elapsed = instant.elapsed();
                debug!(?elapsed, "⏱️ Ping");
            }
        }
        Notification::Blink => {
            info!("🔔 Blink");
        }
        Notification::Registered {
            client_id: id,
            role,
        } => {
            info!(?id, ?role, "✅ Registered");
            if let Ok(mut guard) = client_id.write() {
                *guard = Some(id);
            }
            let _ = tx.send(CommunicationMessage::Registered {
                client_id: id,
                role,
            });
        }
        Notification::ClientConnected { client_id, name } => {
            info!(?client_id, %name, "👋 Client connected");
            let _ = tx.send(CommunicationMessage::ClientConnected { client_id, name });
        }
        Notification::ClientDisconnected { client_id, name } => {
            info!(?client_id, %name, "👋 Client disconnected");
            let _ = tx.send(CommunicationMessage::ClientDisconnected { client_id, name });
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::SlideId;

    use super::*;

    /// Drives one server frame through the dispatcher and returns whatever it
    /// forwarded to the application.
    async fn dispatch(frame: &str) -> Vec<CommunicationMessage> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let last_ping = Arc::new(Mutex::new(None));
        let client_id = Arc::new(RwLock::new(None));

        handle_ws_message(
            Message::Text(frame.into()),
            &tx,
            Arc::clone(&last_ping),
            Arc::clone(&client_id),
        )
        .await;

        drop(tx);
        let mut out = Vec::new();
        while let Some(message) = rx.recv().await {
            out.push(message);
        }
        out
    }

    #[tokio::test]
    async fn a_state_notification_reaches_the_application() {
        let state = State::Running {
            current: SlideId::new(2),
            current_step: 1,
        };
        let frame = serde_json::to_string(&Notification::state(state)).expect("serialize");

        match dispatch(&frame).await.as_slice() {
            [CommunicationMessage::StateChange { state }] => {
                assert_eq!(state.current(), Some(SlideId::new(2)));
            }
            other => panic!("expected one StateChange, got {other:?}"),
        }
    }

    /// A reload is not a state change: clients have to refetch the talk and the
    /// slides, so it must arrive as its own message rather than being folded in.
    #[tokio::test]
    async fn a_talk_change_is_distinguishable_from_a_state_change() {
        let frame =
            serde_json::to_string(&Notification::talk_change(State::default())).expect("serialize");

        assert!(
            matches!(
                dispatch(&frame).await.as_slice(),
                [CommunicationMessage::TalkChange { .. }]
            ),
            "TalkChange must not be delivered as a StateChange"
        );
    }

    /// `Pong` is the heartbeat and carries no application meaning; forwarding it
    /// would wake every client on a timer for nothing.
    #[tokio::test]
    async fn a_pong_is_absorbed() {
        let frame = serde_json::to_string(&Notification::PONG).expect("serialize");
        assert!(dispatch(&frame).await.is_empty());
    }

    /// The registration reply carries the id the server will use for this client
    /// and has to be published, not just recorded internally.
    #[tokio::test]
    async fn registration_publishes_the_assigned_id() {
        // Written as a wire frame rather than built from a `ClientId`: the id is
        // server-assigned and has no public constructor here, which is the point
        // — this checks the id the server sent survives the trip to the app.
        let frame = r#"{"type":"Registered","client_id":{"idx":3,"version":1},"role":"Presenter"}"#;
        let expected = match serde_json::from_str::<Notification>(frame).expect("parse") {
            Notification::Registered { client_id, .. } => client_id,
            other => panic!("fixture is not a Registered frame: {other:?}"),
        };

        match dispatch(frame).await.as_slice() {
            [CommunicationMessage::Registered { client_id, role }] => {
                assert_eq!(*client_id, expected);
                assert_eq!(*role, ClientRole::Presenter);
            }
            other => panic!("expected Registered, got {other:?}"),
        }
    }

    /// The role the server granted has to reach the app, not just the id: a
    /// client that cannot drive the deck needs to say so rather than let the
    /// presenter discover it by pressing a key that does nothing.
    #[tokio::test]
    async fn registration_publishes_the_granted_role() {
        let frame = r#"{"type":"Registered","client_id":{"idx":1,"version":1},"role":"Audience"}"#;
        match dispatch(frame).await.as_slice() {
            [CommunicationMessage::Registered { role, .. }] => {
                assert_eq!(*role, ClientRole::Audience);
            }
            other => panic!("expected Registered, got {other:?}"),
        }
    }

    /// A frame the client cannot parse must not take the connection down with
    /// it — a server that learns a new notification kind should degrade to
    /// ignoring it, not disconnect everyone.
    #[tokio::test]
    async fn an_unparseable_frame_is_ignored() {
        assert!(
            dispatch("{\"type\":\"SomethingFromTheFuture\"}")
                .await
                .is_empty()
        );
        assert!(dispatch("not json at all").await.is_empty());
    }
}
