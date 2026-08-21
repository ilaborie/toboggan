use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo::console::{debug, error, info};
use gloo::net::websocket::Message;
use gloo::net::websocket::futures::WebSocket;
use gloo::timers::callback::{Interval, Timeout};
use js_sys::JSON;
use toboggan_core::timeouts::PING_PERIOD;
use toboggan_core::{Command, Notification};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;

use crate::config::WebSocketConfig;
use crate::services::{CommunicationMessage, ConnectionStatus};
use crate::utils::Timer;
use crate::{play_chime, presenter_token};

/// How often to ping the server, from the constant the server side also reads.
#[expect(
    clippy::cast_possible_truncation,
    reason = "PING_PERIOD is a handful of seconds"
)]
const PING_INTERVAL_MS: u32 = PING_PERIOD.as_millis() as u32;

/// The socket a command is written to, swapped out on every reconnect.
type SharedSink = Rc<RefCell<Option<SplitSink<WebSocket, Message>>>>;

/// Everything about the connection that outlives any single socket.
///
/// Held behind an `Rc` so the reconnect timer and the task reading the socket
/// drive *this* connection rather than a copy: a reconnect has to keep using the
/// application's real command channel, and re-send `Register` down the new
/// socket, or the server — which says nothing at all until it sees that frame —
/// waits forever and the deck stays blank.
struct Connection {
    client_name: String,
    tx_msg: UnboundedSender<CommunicationMessage>,
    tx_cmd: UnboundedSender<Command>,
    sink: SharedSink,
    config: WebSocketConfig,
    retry_count: usize,
    retry_delay: u32,
    ping_interval: Option<Interval>,
}

impl Connection {
    fn send_status(&self, status: ConnectionStatus) {
        debug!("Connection status:", status.to_string());
        let _ = self
            .tx_msg
            .unbounded_send(CommunicationMessage::ConnectionStatusChange { status });
    }

    fn start_pinging(&mut self) {
        self.stop_pinging();

        let tx_cmd = self.tx_cmd.clone();
        let interval = Interval::new(PING_INTERVAL_MS, move || {
            let _timer = Timer::new("ping-latency");
            let _ = tx_cmd.unbounded_send(Command::Ping);
        });

        self.ping_interval = Some(interval);
    }

    fn stop_pinging(&mut self) {
        if let Some(interval) = self.ping_interval.take() {
            interval.cancel();
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.stop_pinging();
    }
}

pub(crate) struct CommunicationService {
    connection: Rc<RefCell<Connection>>,
    /// Taken by the first `connect`, which hands it to the one long-lived task
    /// forwarding commands. Later connects reuse that task.
    rx_cmd: Option<UnboundedReceiver<Command>>,
}

impl CommunicationService {
    pub(crate) fn new(
        client_name: impl Into<String>,
        config: WebSocketConfig,
        tx_msg: UnboundedSender<CommunicationMessage>,
        tx_cmd: UnboundedSender<Command>,
        rx_cmd: UnboundedReceiver<Command>,
    ) -> Self {
        let retry_delay = config.initial_retry_delay.try_into().unwrap_or(1000);
        Self {
            connection: Rc::new(RefCell::new(Connection {
                client_name: client_name.into(),
                config,
                tx_msg,
                tx_cmd,
                sink: Rc::new(RefCell::new(None)),
                retry_count: 0,
                retry_delay,
                ping_interval: None,
            })),
            rx_cmd: Some(rx_cmd),
        }
    }

    pub(crate) fn connect(&mut self) {
        if let Some(rx_cmd) = self.rx_cmd.take() {
            let sink = Rc::clone(&self.connection.borrow().sink);
            spawn_local(forward_commands(rx_cmd, sink));
        }
        connect(&self.connection);
    }

    /// Renames this client, before it connects.
    ///
    /// The name is what `/api/clients` and the connect/disconnect toasts show,
    /// so a presenter can tell the projector, their phone and their own second
    /// window apart.
    pub(crate) fn set_client_name(&mut self, name: &str) {
        name.clone_into(&mut self.connection.borrow_mut().client_name);
    }
}

/// Opens a socket and points the connection's command forwarder at it.
fn connect(connection: &Rc<RefCell<Connection>>) {
    let url = {
        let conn = connection.borrow();
        conn.send_status(ConnectionStatus::Connecting);
        conn.config.url.clone()
    };

    let ws = match WebSocket::open(&url) {
        Ok(ws) => ws,
        Err(err) => {
            error!("Failed to open WebSocket:", err.to_string());
            connection.borrow().send_status(ConnectionStatus::Error {
                message: err.to_string(),
            });
            schedule_reconnect(connection);
            return;
        }
    };

    let (write, read) = ws.split();

    let register = {
        let mut conn = connection.borrow_mut();
        // `WebSocket::open` returns while the socket is still CONNECTING, so
        // this is not yet a working connection — `Connected` is reported once
        // the server actually answers, in `handle_incoming_messages`.
        *conn.sink.borrow_mut() = Some(write);
        conn.start_pinging();

        // Sent on *every* connect, not just the first: the server registers a
        // socket, not a client, so a reconnected socket has to introduce itself
        // again. The token is re-read rather than cached, so a reconnect after
        // the URL changed offers what the URL says now.
        Command::Register {
            name: conn.client_name.clone(),
            token: presenter_token(),
        }
    };
    let _ = connection.borrow().tx_cmd.unbounded_send(register);

    spawn_local(handle_incoming_messages(read, Rc::clone(connection)));
}

fn schedule_reconnect(connection: &Rc<RefCell<Connection>>) {
    let delay = {
        let mut conn = connection.borrow_mut();
        if conn.retry_count >= conn.config.max_retries {
            let message = format!("Max retries ({}) reached", conn.config.max_retries);
            conn.send_status(ConnectionStatus::Error { message });
            return;
        }

        conn.retry_count += 1;
        let delay = conn.retry_delay;

        // Exponential backoff
        let max_delay = conn.config.max_retry_delay.try_into().unwrap_or(30_000);
        conn.retry_delay = (conn.retry_delay * 2).min(max_delay);

        conn.send_status(ConnectionStatus::Reconnecting {
            attempt: conn.retry_count,
            max_attempt: conn.config.max_retries,
            delay: Duration::from_millis(delay.into()),
        });
        delay
    };

    let connection = Rc::clone(connection);
    Timeout::new(delay, move || {
        info!("Attempting reconnection...");
        connect(&connection);
    })
    .forget();
}

/// Writes every command the application sends to whichever socket is open.
///
/// One task for the life of the page, rather than one per socket: the receiver
/// can only be consumed once, so a per-socket task meant that after the first
/// reconnect there was nothing left reading the application's commands, and
/// every keystroke — and the `Register` the server is waiting for — went
/// nowhere.
async fn forward_commands(mut rx_cmd: UnboundedReceiver<Command>, sink: SharedSink) {
    while let Some(cmd) = rx_cmd.next().await {
        let json = serde_wasm_bindgen::to_value(&cmd).unwrap_throw();
        let json_str = JSON::stringify(&json)
            .unwrap_throw()
            .as_string()
            .unwrap_or_default();

        // Taken out for the duration of the send: a `RefCell` borrow cannot be
        // held across an await, and a reconnect may install a new socket while
        // this one is in flight.
        let Some(mut write) = sink.borrow_mut().take() else {
            error!("Dropping a command sent while disconnected:", &json_str);
            continue;
        };

        let sent = write.send(Message::Text(json_str)).await;

        let mut slot = sink.borrow_mut();
        match sent {
            // Only if nothing newer arrived meanwhile — a socket opened while
            // this send was in flight is the one that should survive.
            Ok(()) if slot.is_none() => *slot = Some(write),
            Ok(()) => {}
            // Dropping `write` closes this socket, and the read half will see it
            // and reconnect. The loop keeps going: it has to still be here to
            // serve the socket that replaces this one.
            Err(err) => error!("Failed to send command:", err.to_string()),
        }
    }
}

async fn handle_incoming_messages(
    mut read: SplitStream<WebSocket>,
    connection: Rc<RefCell<Connection>>,
) {
    let mut answered = false;
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                // The first frame back is the proof the handshake completed —
                // `WebSocket::open` returns long before that — so this is where
                // the connection counts as established and the backoff resets.
                if !answered {
                    answered = true;
                    let mut conn = connection.borrow_mut();
                    conn.retry_count = 0;
                    conn.retry_delay = conn.config.initial_retry_delay.try_into().unwrap_or(1000);
                    conn.send_status(ConnectionStatus::Connected);
                }
                let tx_msg = connection.borrow().tx_msg.clone();
                process_message(msg, &tx_msg);
            }
            Err(err) => {
                error!("WebSocket error:", err.to_string());
                connection.borrow().send_status(ConnectionStatus::Error {
                    message: err.to_string(),
                });
                break;
            }
        }
    }

    // Connection closed: drop the dead sink so a command sent before the
    // replacement arrives is reported rather than written into a closed socket.
    {
        let conn = connection.borrow();
        conn.sink.borrow_mut().take();
        conn.send_status(ConnectionStatus::Closed);
    }

    schedule_reconnect(&connection);
}

fn process_message(message: Message, tx: &UnboundedSender<CommunicationMessage>) {
    let text = match message {
        Message::Text(txt) => txt,
        Message::Bytes(bytes) => String::from_utf8_lossy(&bytes).to_string(),
    };

    let json = match JSON::parse(&text) {
        Ok(json) => json,
        Err(err) => {
            error!("Failed to parse message:", err);
            return;
        }
    };

    let notification = match serde_wasm_bindgen::from_value::<Notification>(json) {
        Ok(n) => n,
        Err(err) => {
            error!("Failed to deserialize notification:", err.to_string());
            return;
        }
    };

    match notification {
        Notification::State { state } => {
            let _ = tx.unbounded_send(CommunicationMessage::StateChange { state });
        }
        Notification::TalkChange { state } => {
            let _ = tx.unbounded_send(CommunicationMessage::TalkChange { state });
        }
        Notification::Error { message } => {
            let _ = tx.unbounded_send(CommunicationMessage::Error { error: message });
        }
        Notification::Pong => {
            // Ping response received - timer will be dropped automatically
        }
        Notification::Blink => {
            play_chime();
        }
        Notification::Registered { client_id, role } => {
            info!(
                "Registered with id",
                format!("{client_id:?}"),
                format!("{role:?}")
            );
            let _ = tx.unbounded_send(CommunicationMessage::Registered { client_id, role });
        }
        Notification::ClientConnected { client_id, name } => {
            info!("Client connected:", &name, "id:", format!("{client_id:?}"));
            let _ = tx.unbounded_send(CommunicationMessage::ClientConnected { client_id, name });
        }
        Notification::ClientDisconnected { client_id, name } => {
            info!(
                "Client disconnected:",
                &name,
                "id:",
                format!("{client_id:?}")
            );
            let _ = tx.unbounded_send(CommunicationMessage::ClientDisconnected { client_id, name });
        }
    }
}
