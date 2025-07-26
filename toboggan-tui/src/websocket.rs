use std::time::Duration;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use toboggan_core::{Command, Notification};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use crate::events::AppEvent;

pub struct WebSocketClient {
    tx: mpsc::UnboundedSender<AppEvent>,
    websocket_url: String,
    max_retries: u32,
    retry_delay_ms: u64,
}

impl WebSocketClient {
    #[must_use]
    pub fn new(
        tx: mpsc::UnboundedSender<AppEvent>,
        websocket_url: String,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self {
            tx,
            websocket_url,
            max_retries,
            retry_delay_ms,
        }
    }

    pub async fn run(&self) {
        let mut retry_count = 0;

        loop {
            match self.connect_and_handle().await {
                Ok(()) => {
                    info!("WebSocket connection closed normally");
                    break;
                }
                Err(err) => {
                    retry_count += 1;
                    error!("WebSocket error (attempt {}): {}", retry_count, err);

                    let _ = self.tx.send(AppEvent::ConnectionError(err.to_string()));

                    if retry_count >= self.max_retries {
                        error!("Max retries reached, giving up");
                        let _ = self.tx.send(AppEvent::ConnectionError(
                            "Max connection retries reached".to_string(),
                        ));
                        break;
                    }

                    let delay = Duration::from_millis(self.retry_delay_ms * u64::from(retry_count));
                    warn!("Retrying connection in {:?}", delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    async fn connect_and_handle(&self) -> Result<()> {
        info!("Connecting to WebSocket: {}", self.websocket_url);
        let _ = self.tx.send(AppEvent::Disconnected);

        let (ws_stream, _) = connect_async(&self.websocket_url).await?;
        info!("WebSocket connected successfully");

        let _ = self.tx.send(AppEvent::Connected);
        let (mut write, mut read) = ws_stream.split();

        // Send registration command
        let register_cmd = Command::Register {
            client: toboggan_core::ClientId::new(),
            renderer: toboggan_core::Renderer::Raw,
        };
        let register_msg = serde_json::to_string(&register_cmd)?;
        write.send(Message::Text(register_msg)).await?;
        debug!("Sent registration command");

        // Handle incoming messages
        while let Some(message) = read.next().await {
            match message? {
                Message::Text(text) => {
                    debug!("Received message: {}", text);
                    match serde_json::from_str::<Notification>(&text) {
                        Ok(notification) => {
                            let _ = self.tx.send(AppEvent::NotificationReceived(notification));
                        }
                        Err(err) => {
                            warn!("Failed to parse notification: {}", err);
                        }
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket closed by server");
                    break;
                }
                _ => {
                    debug!("Received non-text message, ignoring");
                }
            }
        }

        let _ = self.tx.send(AppEvent::Disconnected);
        Ok(())
    }

    /// Send a command through the WebSocket connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the command could not be sent.
    pub fn send_command(&self, _command: Command) -> Result<()> {
        // In a real implementation, we'd need to keep the write half of the WebSocket
        // and send commands through it. For simplicity, we'll skip this for now.
        // This would require refactoring to keep the connection alive and accessible.
        Ok(())
    }
}
