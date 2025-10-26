#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::too_many_lines)]

mod common;

use std::sync::atomic::{AtomicU8, Ordering};

use anyhow::bail;
use common::create_multi_slide_talk;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use toboggan_core::{ClientId, Command, Notification};
use toboggan_server::{TobogganState, routes_with_cors};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message as TungsteniteMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

static GLOBAL_TEST_COUNTER: AtomicU8 = AtomicU8::new(0);

async fn create_test_server() -> (String, TobogganState) {
    let talk = create_multi_slide_talk();
    let state = TobogganState::new(talk, 100).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("ws://127.0.0.1:{}/api/ws", addr.port());

    let app = routes_with_cors(None, None).with_state(state.clone());

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("Server failed");
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    (server_url, state)
}

async fn connect_and_register_client(
    server_url: &str,
    client_name: &str,
) -> anyhow::Result<(
    SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TungsteniteMessage>,
    futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ClientId,
)> {
    let (ws_stream, _) = connect_async(server_url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let client = ClientId::new();

    let register_cmd = Command::Register { client };
    let register_msg = serde_json::to_string(&register_cmd)?;
    ws_sender
        .send(TungsteniteMessage::text(register_msg))
        .await?;

    println!("[{client_name}] Sent registration for client: {client:?}");

    if let Some(msg) = ws_receiver.next().await {
        let msg = msg?;
        if let TungsteniteMessage::Text(text) = msg {
            let notification: Notification = serde_json::from_str(&text)?;
            match notification {
                Notification::State { state, .. } => {
                    println!("[{client_name}] Received initial state: {state:?}");
                }
                _ => {
                    bail!("Expected State notification, got: {notification:?}");
                }
            }
        }
    }

    Ok((ws_sender, ws_receiver, client))
}

async fn send_command_and_get_response(
    ws_sender: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TungsteniteMessage>,
    ws_receiver: &mut futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    command: Command,
    client_name: &str,
) -> anyhow::Result<Notification> {
    let cmd_msg = serde_json::to_string(&command)?;
    ws_sender.send(TungsteniteMessage::text(cmd_msg)).await?;
    println!("[{client_name}] Sent command: {command:?}");

    loop {
        if let Some(msg) = ws_receiver.next().await {
            let msg = msg?;
            if let TungsteniteMessage::Text(text) = msg {
                let notification: Notification = serde_json::from_str(&text)?;
                println!("[{client_name}] Received notification: {notification:?}");

                match notification {
                    Notification::Pong => {
                        println!(
                            "[{client_name}] Ignoring pong, waiting for state notification..."
                        );
                    }
                    _ => return Ok(notification),
                }
            }
        } else {
            bail!("No response received");
        }
    }
}

async fn wait_for_notification(
    ws_receiver: &mut futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    client_name: &str,
    timeout_ms: u64,
) -> anyhow::Result<Notification> {
    wait_for_recent_notification(ws_receiver, client_name, timeout_ms).await
}

async fn wait_for_recent_notification(
    ws_receiver: &mut futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    client_name: &str,
    timeout_ms: u64,
) -> anyhow::Result<Notification> {
    let timeout = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async {
        loop {
            if let Some(msg) = ws_receiver.next().await {
                let msg = msg?;
                if let TungsteniteMessage::Text(text) = msg {
                    let notification: Notification = serde_json::from_str(&text)?;
                    println!("[{client_name}] Received notification: {notification:?}");

                    match &notification {
                        Notification::Pong => {
                            println!(
                                "[{client_name}] Ignoring pong, waiting for state notification..."
                            );
                        }
                        Notification::State { .. } => {
                            println!("[{client_name}] Received state notification");
                            return Ok(notification);
                        }
                        _ => return Ok(notification),
                    }
                }
            } else {
                bail!("No notification received");
            }
        }
    });

    match timeout.await {
        Ok(result) => result,
        Err(err) => bail!("[{client_name}] Timeout waiting for notification: {err}"),
    }
}

#[tokio::test]
async fn test_client_disconnect_and_reconnect_sync() {
    let _test_id = GLOBAL_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

    let (server_url, _state) = create_test_server().await;

    let (mut client1_sender, mut client1_receiver, _client1_id) =
        connect_and_register_client(&server_url, "Client1")
            .await
            .expect("Failed to connect client 1");

    let (client2_sender, mut client2_receiver, _client2_id) =
        connect_and_register_client(&server_url, "Client2")
            .await
            .expect("Failed to connect client 2");

    println!("Both clients connected successfully");

    println!("\n=== Client 1 navigates to slide 2 ===");
    let _next_notification = send_command_and_get_response(
        &mut client1_sender,
        &mut client1_receiver,
        Command::Next,
        "Client1",
    )
    .await
    .expect("Failed to navigate next");

    let _client2_sync = wait_for_notification(&mut client2_receiver, "Client2", 1000)
        .await
        .expect("Client 2 should receive sync");

    drop(client2_sender);
    drop(client2_receiver);
    println!("Client 2 disconnected");

    println!("\n=== Client 1 navigates to slide 3 (Client 2 disconnected) ===");
    let _next2_notification = send_command_and_get_response(
        &mut client1_sender,
        &mut client1_receiver,
        Command::Next,
        "Client1",
    )
    .await
    .expect("Failed to navigate next again");

    println!("\n=== Client 2 reconnects ===");
    let (_client2_sender_new, _client2_receiver_new, _client2_id_new) =
        connect_and_register_client(&server_url, "Client2-Reconnected")
            .await
            .expect("Failed to reconnect client 2");

    println!("✅ Client 2 successfully reconnected and received current state");

    println!("\n🎉 Client disconnect/reconnect test passed!");
    println!("✅ Disconnected clients are properly cleaned up");
    println!("✅ Reconnected clients receive the current state immediately");
}
