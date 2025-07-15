// Re-export toboggan-client types
pub use toboggan_client::{CommunicationMessage, WebSocketClient};
use toboggan_client::{ConnectionStatus, TobogganConfig};
use toboggan_core::{ClientConfig, Command, Notification};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::events::AppEvent;

pub struct ConnectionHandler {
    config: TobogganConfig,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    command_tx: Option<mpsc::UnboundedSender<Command>>,
}

impl ConnectionHandler {
    #[must_use]
    pub fn new(config: TobogganConfig, event_tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            config,
            event_tx,
            command_tx: None,
        }
    }

    /// Start WebSocket client connection using toboggan-client
    pub fn start(&mut self) {
        info!("Starting WebSocket client using toboggan-client");

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        self.command_tx = Some(command_tx.clone());

        let client_id = self.config.client_id();
        let websocket_config = self.config.websocket();

        let (mut ws_client, mut message_rx) =
            WebSocketClient::new(command_tx.clone(), command_rx, client_id, websocket_config);

        let event_tx = self.event_tx.clone();
        let _ = event_tx.send(AppEvent::ConnectionStatus(ConnectionStatus::Closed));

        // Start the WebSocket client
        tokio::spawn(async move {
            ws_client.connect().await;
        });

        // Handle messages from the WebSocket client
        let event_tx_clone = self.event_tx.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                match message {
                    CommunicationMessage::ConnectionStatusChange { status } => {
                        let _ = event_tx_clone.send(AppEvent::ConnectionStatus(status));
                    }
                    CommunicationMessage::StateChange { state } => {
                        let _ = event_tx_clone.send(AppEvent::NotificationReceived(
                            Notification::State { state },
                        ));
                    }
                    CommunicationMessage::TalkChange { state } => {
                        tracing::info!("📝 Presentation updated");
                        let _ = event_tx_clone.send(AppEvent::NotificationReceived(
                            Notification::TalkChange { state },
                        ));
                    }
                    CommunicationMessage::Error { error } => {
                        let _ = event_tx_clone.send(AppEvent::Error(error));
                    }
                }
            }
        });
    }

    /// Send a command through the WebSocket connection
    pub fn send_command(&self, command: &Command) {
        if let Some(command_tx) = &self.command_tx {
            if let Err(err) = command_tx.send(command.clone()) {
                error!("Failed to send command through WebSocket: {err}");
            }
        } else {
            error!("WebSocket command channel not available");
        }
    }
}
