use std::time::Duration;

use toboggan_client::{TobogganApi, TobogganConfig, WebSocketClient};
use toboggan_core::{ClientConfig, Command};
use tokio::sync::mpsc;
use tracing::info;

#[allow(clippy::expect_used)]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().pretty().init();

    let config = TobogganConfig::default();
    let client_id = config.client_id();
    let api_url = config.api_url();
    let websocket = config.websocket();

    let api = TobogganApi::new(api_url);

    let (tx, rx) = mpsc::unbounded_channel();
    let (mut com, mut rx_msg) = WebSocketClient::new(tx.clone(), rx, client_id, websocket);

    let read_messages = tokio::spawn(async move {
        while let Some(msg) = rx_msg.recv().await {
            info!(?msg, "receive WS message");
        }
    });

    let talk = api.talk().await.expect("loading talk");
    info!(?talk, "🎙️ Talk");

    com.connect().await;

    for _ in 0..10 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        tx.send(Command::Next).expect("sending next");
    }

    let _ = tokio::time::timeout(Duration::from_secs(20), read_messages).await;
    info!("👋 Bye");
}
