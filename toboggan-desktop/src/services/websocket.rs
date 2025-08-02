use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use toboggan_core::{ClientId, Command, Notification, Renderer};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{error, info, warn};

use crate::messages::Message;

pub struct WebSocketClient {
    sender: Option<mpsc::UnboundedSender<Command>>,
    client_id: ClientId,
}

impl WebSocketClient {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            sender: None,
            client_id,
        }
    }

    pub async fn connect(&mut self, url: &str) -> Result<mpsc::UnboundedReceiver<Message>> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let (cmd_sender, mut cmd_receiver) = mpsc::unbounded_channel::<Command>();
        let (msg_sender, msg_receiver) = mpsc::unbounded_channel::<Message>();

        self.sender = Some(cmd_sender);

        // Send registration command
        let register_cmd = Command::Register {
            client: self.client_id,
            renderer: Renderer::Html,
        };
        let register_msg = serde_json::to_string(&register_cmd)?;
        ws_sender.send(WsMessage::Text(register_msg.into())).await?;

        let msg_sender_clone = msg_sender.clone();

        // Spawn task to handle outgoing commands
        tokio::spawn(async move {
            while let Some(command) = cmd_receiver.recv().await {
                match serde_json::to_string(&command) {
                    Ok(msg) => {
                        if let Err(send_error) = ws_sender.send(WsMessage::Text(msg.into())).await {
                            error!("Failed to send WebSocket message: {}", send_error);
                            let _ = msg_sender_clone
                                .send(Message::WebSocketError(send_error.to_string()));
                            break;
                        }
                    }
                    Err(serialize_error) => {
                        error!("Failed to serialize command: {}", serialize_error);
                        let _ = msg_sender_clone
                            .send(Message::WebSocketError(serialize_error.to_string()));
                    }
                }
            }
        });

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            let _ = msg_sender.send(Message::WebSocketConnected);

            while let Some(message) = ws_receiver.next().await {
                match message {
                    Ok(WsMessage::Text(text)) => {
                        match serde_json::from_str::<Notification>(&text) {
                            Ok(notification) => {
                                if msg_sender
                                    .send(Message::NotificationReceived(notification))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(parse_error) => {
                                warn!("Failed to parse notification: {}", parse_error);
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("WebSocket connection closed");
                        let _ = msg_sender.send(Message::WebSocketDisconnected);
                        break;
                    }
                    Err(ws_error) => {
                        error!("WebSocket error: {}", ws_error);
                        let _ = msg_sender.send(Message::WebSocketError(ws_error.to_string()));
                        break;
                    }
                    _ => {
                        // Ignore other message types
                    }
                }
            }
        });

        Ok(msg_receiver)
    }

    pub fn send_command(&self, command: Command) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send(command)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.sender.is_some()
    }
}
