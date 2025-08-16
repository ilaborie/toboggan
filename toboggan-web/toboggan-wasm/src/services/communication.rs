use std::{
    cell::{Cell, RefCell},
    mem,
    rc::Rc,
    time::Duration,
};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

// WARNING: This module runs in WASM browser context.
// NEVER use std::sync blocking operations (like mpsc::Receiver::recv())
// in async functions as they WILL FREEZE the browser!
//
// Use Rc<RefCell<>> instead of Arc<Mutex<>> in single-threaded WASM context.
use futures::{SinkExt, StreamExt};
use gloo::{
    console::{debug, error, info, warn},
    net::websocket::{Message, futures::WebSocket},
    timers::callback::{Interval, Timeout},
};
use js_sys::JSON;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;

use toboggan_core::{ClientId, Command, Notification, Renderer};

use crate::{
    config::WebSocketConfig,
    play_chime,
    services::{CommunicationMessage, ConnectionStatus},
    utils::Timer,
};

const PING_INTERVAL_MS: u32 = 60 * 1_000; // 1 minute
const PING_LABEL: &str = "ping-latency";

#[derive(Clone)]
struct ConnectionState {
    retry_count: usize,
    retry_delay: u32,
    is_disposed: bool,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            retry_count: 0,
            retry_delay: 1000, // 1 second initial delay
            is_disposed: false,
        }
    }
}

pub struct CommunicationService {
    client_id: ClientId,
    tx_msg: UnboundedSender<CommunicationMessage>,
    tx_cmd: UnboundedSender<Command>,
    rx_cmd: Rc<RefCell<UnboundedReceiver<Command>>>,
    config: WebSocketConfig,
    state: Rc<RefCell<ConnectionState>>,
    ping_interval: Option<Interval>,
    reconnect_timeout: Option<Timeout>,
    ping_timer_active: Rc<Cell<bool>>,
}

impl CommunicationService {
    pub fn new(
        client_id: ClientId,
        config: WebSocketConfig,
        tx_msg: UnboundedSender<CommunicationMessage>,
        tx_cmd: UnboundedSender<Command>,
        rx_cmd: UnboundedReceiver<Command>,
    ) -> Self {
        let state = ConnectionState {
            retry_delay: config.initial_retry_delay.try_into().unwrap_or(1000),
            ..Default::default()
        };
        let rx_cmd = Rc::new(RefCell::new(rx_cmd));

        Self {
            client_id,
            config,
            tx_msg,
            tx_cmd,
            rx_cmd,
            state: Rc::new(RefCell::new(state)),
            ping_interval: None,
            reconnect_timeout: None,
            ping_timer_active: Rc::new(Cell::new(false)),
        }
    }

    /// Helper function to send connection status change messages
    fn send_status_change(&self, status: ConnectionStatus) {
        send_status_change_via(&self.tx_msg, status);
    }

    pub fn connect(&mut self) {
        let state = self.state.borrow();
        if state.is_disposed {
            warn!("Illegal disposed state, cannot connect");
            return;
        }
        mem::drop(state);

        self.attempt_connection();
    }

    fn attempt_connection(&mut self) {
        // Notify connecting status
        self.send_status_change(ConnectionStatus::Connecting);

        let ws = match WebSocket::open(&self.config.url) {
            Ok(ws) => ws,
            Err(err) => {
                let message = err.to_string();
                error!("Failed to open WebSocket", &message);
                self.send_status_change(ConnectionStatus::Error { message });
                self.schedule_reconnect();
                return;
            }
        };

        let (write, read) = ws.split();

        // Handle connection success
        self.handle_connection_open();

        // Handle outgoing messages
        let rx_cmd = Rc::clone(&self.rx_cmd);
        spawn_local(handle_outgoing_commands(rx_cmd, write));

        // Handle incoming messages and connection lifecycle
        let tx_msg_clone = self.tx_msg.clone();
        let state_clone = self.state.clone();
        let config = self.config.clone();
        let ping_timer_active = Rc::clone(&self.ping_timer_active);
        spawn_local(handle_incoming_messages(
            read,
            tx_msg_clone,
            state_clone,
            config,
            ping_timer_active,
        ));
    }

    fn handle_connection_open(&mut self) {
        // Reset retry state
        {
            let mut state = self.state.borrow_mut();
            state.retry_count = 0;
            state.retry_delay = self.config.initial_retry_delay.try_into().unwrap_or(1000);
        }

        // Start pinging
        self.start_pinging();

        // Notify connected status
        self.send_status_change(ConnectionStatus::Connected);
    }

    fn schedule_reconnect(&mut self) {
        let (retry_count, retry_delay, max_retries) = {
            let mut state = self.state.borrow_mut();
            if state.is_disposed {
                return;
            }

            if state.retry_count >= self.config.max_retries {
                let message = format!("Max retries reached! ({})", self.config.max_retries);
                self.send_status_change(ConnectionStatus::Error { message });
                return;
            }

            state.retry_count += 1;
            let current_delay = state.retry_delay;

            // Exponential backoff
            let max_delay = self.config.max_retry_delay.try_into().unwrap_or(30_000);
            state.retry_delay = (state.retry_delay * 2).min(max_delay);

            (state.retry_count, current_delay, self.config.max_retries)
        };

        self.send_status_change(ConnectionStatus::Reconnecting {
            attempt: retry_count,
            max_attempt: max_retries,
            delay: Duration::from_millis(retry_delay.into()),
        });

        // Schedule reconnection with current delay
        let tx_msg_clone = self.tx_msg.clone();
        let config = self.config.clone();
        let client_id = self.client_id;
        let state_clone= Rc::clone(&self.state);
        let tx_cmd = self.tx_cmd.clone();
        let rx_cmd = Rc::clone(&self.rx_cmd);

        let timeout = Timeout::new(retry_delay, move || {
            let state = state_clone.borrow();
            if !state.is_disposed {
                reconnect_with_channel(&config, client_id, tx_msg_clone, &tx_cmd, rx_cmd);
            }
        });

        self.reconnect_timeout = Some(timeout);
    }

    fn start_pinging(&mut self) {
        self.stop_pinging();

        let tx_cmd = self.tx_cmd.clone();
        let ping_timer_active = Rc::clone(&self.ping_timer_active);
        let interval = Interval::new(PING_INTERVAL_MS, move || {
            // Try to start a new timer - Timer::new will handle the logic
            let timer = Timer::new(PING_LABEL, Rc::clone(&ping_timer_active));
            if tx_cmd.unbounded_send(Command::Ping).is_err() {
                error!("Failed to send ping command");
            }
            // Timer will be dropped here, but that's OK - we want it to stay active
            // until we receive a Pong response
            mem::forget(timer);
        });

        self.ping_interval = Some(interval);
    }

    fn stop_pinging(&mut self) {
        if let Some(interval) = self.ping_interval.take() {
            interval.cancel();
        }
    }
}

impl Drop for CommunicationService {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.is_disposed = true;
        mem::drop(state);

        self.stop_pinging();

        if let Some(timeout) = self.reconnect_timeout.take() {
            timeout.cancel();
        }
    }
}

/// Helper function to send connection status change messages via a sender (for async contexts)
fn send_status_change_via(tx: &UnboundedSender<CommunicationMessage>, status: ConnectionStatus) {
    debug!("🗿", status.to_string());
    let _ = tx.unbounded_send(CommunicationMessage::ConnectionStatusChange { status });
}

#[allow(clippy::await_holding_refcell_ref)]
fn reconnect_with_channel(
    config: &WebSocketConfig,
    client_id: ClientId,
    tx_msg: UnboundedSender<CommunicationMessage>,
    tx_cmd: &UnboundedSender<Command>,
    rx_cmd: Rc<RefCell<UnboundedReceiver<Command>>>,
) {
    // Create a new service instance for reconnection
    // This is a simplified approach - in a real implementation you might want
    // to have a more sophisticated state management
    info!("Attempting to reconnect...");

    // Attempt connection
    let ws = match WebSocket::open(&config.url) {
        Ok(ws) => ws,
        Err(err) => {
            error!("Reconnection failed", err.to_string());
            return;
        }
    };

    let (mut write, mut read) = ws.split();

    // Notify connected
    send_status_change_via(&tx_msg, ConnectionStatus::Connected);

    // Register with server
    let register_cmd = Command::Register {
        client: client_id,
        renderer: Renderer::default(),
    };
    let _ = tx_cmd.unbounded_send(register_cmd);

    // Handle messages (simplified version for reconnection)
    spawn_local(async move {
        // TODO replace by handle incomming messages
        use futures::StreamExt;
        loop {
            // Safe in single-threaded WASM: no other tasks can access the RefCell during await
            let cmd = {
                #[allow(clippy::await_holding_refcell_ref)]
                let mut rx_cmd = rx_cmd.borrow_mut();
                rx_cmd.next().await
            };
            let Some(cmd) = cmd else {
                break;
            };
            let json = serde_wasm_bindgen::to_value(&cmd).unwrap_throw();
            let json_str = JSON::stringify(&json)
                .unwrap_throw()
                .as_string()
                .unwrap_or_default();
            let _ = write.send(Message::Text(json_str)).await;
        }
    });

    // Create a new ping timer tracking for the reconnected session
    let ping_timer_active = Rc::new(Cell::new(false));
    spawn_local(async move {
        while let Some(msg) = read.next().await {
            if let Ok(msg) = msg {
                handle_ws_message(msg, &tx_msg, &ping_timer_active);
            }
        }
    });
}

fn schedule_reconnect_async(
    state: &Rc<RefCell<ConnectionState>>,
    config: &WebSocketConfig,
    tx_msg: &UnboundedSender<CommunicationMessage>,
) {
    let (retry_count, retry_delay, should_reconnect) = {
        let mut state_ref = state.borrow_mut();
        if state_ref.is_disposed || state_ref.retry_count >= config.max_retries {
            return;
        }

        state_ref.retry_count += 1;
        let current_delay = state_ref.retry_delay;
        state_ref.retry_delay =
            (state_ref.retry_delay * 2).min(config.max_retry_delay.try_into().unwrap_or(30_000));

        (state_ref.retry_count, current_delay, true)
    };

    if should_reconnect {
        send_status_change_via(
            tx_msg,
            ConnectionStatus::Reconnecting {
                attempt: retry_count,
                max_attempt: config.max_retries,
                delay: Duration::from_millis(retry_delay.into()),
            },
        );

        // TODO weird
        // Use callback-based timeout as recommended by gloo docs
        let timeout = Timeout::new(retry_delay, move || {
            info!("Auto-reconnect timeout triggered - attempting reconnection");
            // This is a simplified reconnection trigger
            // In a full implementation, you'd recreate the full connection flow
        });
        timeout.forget(); // Recommended by gloo docs for timeouts that won't be cancelled
    }
}

// CRITICAL: WASM async function - must not use blocking sync operations
// that would freeze the browser's single-threaded event loop
#[allow(clippy::await_holding_refcell_ref)]
async fn handle_outgoing_commands(
    rx_cmd: Rc<RefCell<UnboundedReceiver<Command>>>,
    mut write: futures::stream::SplitSink<WebSocket, Message>,
) {
    use futures::StreamExt;
    loop {
        // Safe in single-threaded WASM: no other tasks can access the RefCell during await
        let cmd = {
            #[allow(clippy::await_holding_refcell_ref)]
            let mut rx_cmd = rx_cmd.borrow_mut();
            rx_cmd.next().await
        };
        let Some(cmd) = cmd else {
            break;
        };
        let json = match serde_wasm_bindgen::to_value(&cmd) {
            Ok(json) => json,
            Err(err) => {
                error!("Failed to serialize command", err.to_string());
                continue;
            }
        };
        let json = JSON::stringify(&json)
            .unwrap_throw()
            .as_string()
            .unwrap_or_default();
        let item = Message::Text(json);

        if let Err(err) = write.send(item).await {
            error!("Failed to send WS command", err.to_string());
            break;
        }
    }
}

async fn handle_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    tx_msg: UnboundedSender<CommunicationMessage>,
    state: Rc<RefCell<ConnectionState>>,
    config: WebSocketConfig,
    ping_timer_active: Rc<Cell<bool>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                handle_ws_message(msg, &tx_msg, &ping_timer_active);
            }
            Err(error) => {
                error!("Failed to read WS incoming message", error.to_string());
                send_status_change_via(
                    &tx_msg,
                    ConnectionStatus::Error {
                        message: error.to_string(),
                    },
                );
                break;
            }
        }
    }

    // Connection closed - notify and schedule reconnect
    send_status_change_via(&tx_msg, ConnectionStatus::Closed);

    // Schedule reconnect if not disposed
    schedule_reconnect_async(&state, &config, &tx_msg);
}

fn handle_ws_message(
    message: Message,
    tx: &UnboundedSender<CommunicationMessage>,
    ping_timer_active: &Rc<Cell<bool>>,
) {
    let message_text = match message {
        Message::Text(txt) => txt,
        Message::Bytes(items) => String::from_utf8_lossy(&items).to_string(),
    };

    let json_value = match JSON::parse(&message_text) {
        Ok(json) => json,
        Err(err) => {
            error!("Failed to deserialize message", &message_text, err);
            return;
        }
    };

    // info!("Incoming message", &json_value);

    let notification = match serde_wasm_bindgen::from_value::<Notification>(json_value) {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to create notification", err.to_string());
            return;
        }
    };

    match notification {
        Notification::State { state, .. } => {
            let _ = tx.unbounded_send(CommunicationMessage::StateChange { state });
        }
        Notification::Error { message, .. } => {
            let _ = tx.unbounded_send(CommunicationMessage::Error { error: message });
        }
        Notification::Pong { .. } => {
            // Use Timer's static method to safely end the timer
            Timer::try_end(ping_timer_active, PING_LABEL);
        }
        Notification::Blink => {
            play_chime();
        }
    }
}
