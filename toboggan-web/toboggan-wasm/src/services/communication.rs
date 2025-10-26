use std::time::Duration;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::{SinkExt, StreamExt};
use gloo::console::{debug, error, info};
use gloo::net::websocket::Message;
use gloo::net::websocket::futures::WebSocket;
use gloo::timers::callback::{Interval, Timeout};
use js_sys::JSON;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;

use toboggan_core::{ClientId, Command, Notification};

use crate::config::WebSocketConfig;
use crate::play_chime;
use crate::services::{CommunicationMessage, ConnectionStatus};
use crate::utils::Timer;

const PING_INTERVAL_MS: u32 = 60_000; // 1 minute

pub struct CommunicationService {
    client_id: ClientId,
    tx_msg: UnboundedSender<CommunicationMessage>,
    tx_cmd: UnboundedSender<Command>,
    rx_cmd: Option<UnboundedReceiver<Command>>,
    config: WebSocketConfig,
    retry_count: usize,
    retry_delay: u32,
    ping_interval: Option<Interval>,
}

impl CommunicationService {
    pub fn new(
        client_id: ClientId,
        config: WebSocketConfig,
        tx_msg: UnboundedSender<CommunicationMessage>,
        tx_cmd: UnboundedSender<Command>,
        rx_cmd: UnboundedReceiver<Command>,
    ) -> Self {
        let retry_delay = config.initial_retry_delay.try_into().unwrap_or(1000);
        Self {
            client_id,
            config,
            tx_msg,
            tx_cmd,
            rx_cmd: Some(rx_cmd),
            retry_count: 0,
            retry_delay,
            ping_interval: None,
        }
    }

    pub fn connect(&mut self) {
        self.send_status(ConnectionStatus::Connecting);

        let ws = match WebSocket::open(&self.config.url) {
            Ok(ws) => ws,
            Err(err) => {
                error!("Failed to open WebSocket:", err.to_string());
                self.send_status(ConnectionStatus::Error {
                    message: err.to_string(),
                });
                self.schedule_reconnect();
                return;
            }
        };

        let (write, read) = ws.split();

        // Reset retry state on successful connection
        self.retry_count = 0;
        self.retry_delay = self.config.initial_retry_delay.try_into().unwrap_or(1000);

        self.start_pinging();
        self.send_status(ConnectionStatus::Connected);

        // Handle outgoing messages
        if let Some(rx_cmd) = self.rx_cmd.take() {
            spawn_local(handle_outgoing_commands(rx_cmd, write));
        }

        // Handle incoming messages
        let tx_msg = self.tx_msg.clone();
        let client_id = self.client_id;
        let config = self.config.clone();
        spawn_local(async move {
            handle_incoming_messages(read, tx_msg, client_id, config).await;
        });
    }

    fn send_status(&self, status: ConnectionStatus) {
        debug!("Connection status:", status.to_string());
        let _ = self
            .tx_msg
            .unbounded_send(CommunicationMessage::ConnectionStatusChange { status });
    }

    fn schedule_reconnect(&mut self) {
        if self.retry_count >= self.config.max_retries {
            self.send_status(ConnectionStatus::Error {
                message: format!("Max retries ({}) reached", self.config.max_retries),
            });
            return;
        }

        self.retry_count += 1;
        let delay = self.retry_delay;

        // Exponential backoff
        self.retry_delay =
            (self.retry_delay * 2).min(self.config.max_retry_delay.try_into().unwrap_or(30_000));

        self.send_status(ConnectionStatus::Reconnecting {
            attempt: self.retry_count,
            max_attempt: self.config.max_retries,
            delay: Duration::from_millis(delay.into()),
        });

        let mut service = Self::new(
            self.client_id,
            self.config.clone(),
            self.tx_msg.clone(),
            self.tx_cmd.clone(),
            // Create a new receiver channel for reconnection
            futures::channel::mpsc::unbounded().1,
        );

        Timeout::new(delay, move || {
            info!("Attempting reconnection...");
            service.connect();
        })
        .forget();
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

impl Drop for CommunicationService {
    fn drop(&mut self) {
        self.stop_pinging();
    }
}

async fn handle_outgoing_commands(
    mut rx_cmd: UnboundedReceiver<Command>,
    mut write: futures::stream::SplitSink<WebSocket, Message>,
) {
    while let Some(cmd) = rx_cmd.next().await {
        let json = serde_wasm_bindgen::to_value(&cmd).unwrap_throw();
        let json_str = JSON::stringify(&json)
            .unwrap_throw()
            .as_string()
            .unwrap_or_default();

        if write.send(Message::Text(json_str)).await.is_err() {
            error!("Failed to send command");
            break;
        }
    }
}

async fn handle_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    tx_msg: UnboundedSender<CommunicationMessage>,
    client_id: ClientId,
    config: WebSocketConfig,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => process_message(msg, &tx_msg),
            Err(err) => {
                error!("WebSocket error:", err.to_string());
                tx_msg
                    .unbounded_send(CommunicationMessage::ConnectionStatusChange {
                        status: ConnectionStatus::Error {
                            message: err.to_string(),
                        },
                    })
                    .ok();
                break;
            }
        }
    }

    // Connection closed
    tx_msg
        .unbounded_send(CommunicationMessage::ConnectionStatusChange {
            status: ConnectionStatus::Closed,
        })
        .ok();

    // Schedule reconnection
    let mut service = CommunicationService::new(
        client_id,
        config,
        tx_msg,
        futures::channel::mpsc::unbounded().0,
        futures::channel::mpsc::unbounded().1,
    );
    service.schedule_reconnect();
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
        Notification::State { state, .. } => {
            let _ = tx.unbounded_send(CommunicationMessage::StateChange { state });
        }
        Notification::Error { message, .. } => {
            let _ = tx.unbounded_send(CommunicationMessage::Error { error: message });
        }
        Notification::Pong => {
            // Ping response received - timer will be dropped automatically
        }
        Notification::Blink => {
            play_chime();
        }
    }
}
