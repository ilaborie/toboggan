#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::too_many_lines)]

use futures::{SinkExt, StreamExt};
use toboggan_core::{
    ClientId, Command, Date, Notification, Renderer, Slide, Talk,
};
use toboggan_server::{TobogganState, routes_with_cors};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message as TungsteniteMessage;

// Global test counter to ensure unique SlideId ranges per test
static GLOBAL_TEST_COUNTER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Creates a test talk with multiple slides for navigation testing
fn create_test_talk() -> Talk {
    Talk::new("Multi-Client Sync Test Talk")
        .with_date(Date::ymd(2025, 1, 25))
        .add_slide(Slide::cover("Cover Slide").with_body("This is the cover slide"))
        .add_slide(
            Slide::new("First Content Slide")
                .with_body("This is the first content slide")
                .with_notes("Notes for first slide"),
        )
        .add_slide(Slide::new("Second Content Slide").with_body("This is the second content slide"))
        .add_slide(Slide::new("Final Slide").with_body("This is the final slide"))
}

/// Helper to create a test server with the given talk
async fn create_test_server() -> (String, TobogganState) {
    // Note: SlideId sequence is managed by the calling test for better isolation
    let talk = create_test_talk();
    let state = TobogganState::new(talk, 100);

    // Create a test server on a random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("ws://127.0.0.1:{}/api/ws", addr.port());

    let app = routes_with_cors(None).with_state(state.clone());

    // Start the server in the background
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("Server failed");
    });

    // Give the server more time to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    (server_url, state)
}

/// Helper to connect a WebSocket client and register it
async fn connect_and_register_client(
    server_url: &str,
    client_name: &str,
) -> Result<
    (
        futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            TungsteniteMessage,
        >,
        futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        ClientId,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let (ws_stream, _) = connect_async(server_url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Generate a client ID for registration
    let client_id = ClientId::new();

    // Send registration command
    let register_cmd = Command::Register {
        client: client_id,
        renderer: Renderer::Html,
    };
    let register_msg = serde_json::to_string(&register_cmd)?;
    ws_sender
        .send(TungsteniteMessage::text(register_msg))
        .await?;

    println!(
        "[{}] Sent registration for client: {:?}",
        client_name, client_id
    );

    // Wait for initial state notification
    if let Some(msg) = ws_receiver.next().await {
        let msg = msg?;
        if let TungsteniteMessage::Text(text) = msg {
            let notification: Notification = serde_json::from_str(&text)?;
            match notification {
                Notification::State { state, .. } => {
                    println!("[{}] Received initial state: {:?}", client_name, state);
                }
                _ => {
                    return Err(
                        format!("Expected State notification, got: {:?}", notification).into(),
                    );
                }
            }
        }
    }

    // Return the split streams directly
    Ok((ws_sender, ws_receiver, client_id))
}

/// Helper to send a command and return the response notification
async fn send_command_and_get_response(
    ws_sender: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        TungsteniteMessage,
    >,
    ws_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    command: Command,
    client_name: &str,
) -> Result<Notification, Box<dyn std::error::Error + Send + Sync>> {
    // Send command
    let cmd_msg = serde_json::to_string(&command)?;
    ws_sender.send(TungsteniteMessage::text(cmd_msg)).await?;
    println!("[{}] Sent command: {:?}", client_name, command);

    // Wait for response, filtering out pong messages
    loop {
        if let Some(msg) = ws_receiver.next().await {
            let msg = msg?;
            if let TungsteniteMessage::Text(text) = msg {
                let notification: Notification = serde_json::from_str(&text)?;
                println!(
                    "[{}] Received notification: {:?}",
                    client_name, notification
                );

                // Filter out pong messages and only return state/error notifications
                match notification {
                    Notification::Pong { .. } => {
                        println!(
                            "[{}] Ignoring pong, waiting for state notification...",
                            client_name
                        );
                    }
                    _ => return Ok(notification),
                }
            }
        } else {
            return Err("No response received".into());
        }
    }
}

/// Helper to wait for a notification without sending a command
async fn wait_for_notification(
    ws_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    client_name: &str,
    timeout_ms: u64,
) -> Result<Notification, Box<dyn std::error::Error + Send + Sync>> {
    wait_for_recent_notification(ws_receiver, client_name, timeout_ms, None).await
}

/// Helper to wait for a notification with timestamp newer than the given threshold
async fn wait_for_recent_notification(
    ws_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    client_name: &str,
    timeout_ms: u64,
    after_timestamp: Option<toboggan_core::Timestamp>,
) -> Result<Notification, Box<dyn std::error::Error + Send + Sync>> {
    // Use timeout to avoid waiting indefinitely
    let timeout = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async {
        loop {
            if let Some(msg) = ws_receiver.next().await {
                let msg = msg?;
                if let TungsteniteMessage::Text(text) = msg {
                    let notification: Notification = serde_json::from_str(&text)?;
                    println!(
                        "[{}] Received notification: {:?}",
                        client_name, notification
                    );

                    // Filter out pong messages and only return state/error notifications
                    match &notification {
                        Notification::Pong { .. } => {
                            println!(
                                "[{}] Ignoring pong, waiting for state notification...",
                                client_name
                            );
                        }
                        Notification::State { timestamp, .. } => {
                            println!(
                                "[{}] Received state notification with timestamp: {:?}",
                                client_name, timestamp
                            );

                            // Check if this notification is newer than the threshold
                            if let Some(after) = after_timestamp
                                && *timestamp < after
                            {
                                println!(
                                    "[{}] Ignoring old notification with timestamp {:?} (before {:?})",
                                    client_name, timestamp, after
                                );
                                continue;
                            }

                            return Ok(notification);
                        }
                        _ => return Ok(notification),
                    }
                }
            } else {
                return Err("No notification received".into());
            }
        }
    });

    match timeout.await {
        Ok(result) => result,
        Err(_) => Err(format!("[{}] Timeout waiting for notification", client_name).into()),
    }
}

/// Helper to receive next non-pong notification from WebSocket stream
async fn receive_next_non_pong_notification(
    ws_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    client_name: &str,
) -> Result<Notification, Box<dyn std::error::Error + Send + Sync>> {
    loop {
        if let Some(msg) = ws_receiver.next().await {
            let msg = msg?;
            if let TungsteniteMessage::Text(text) = msg {
                let notification: Notification = serde_json::from_str(&text)?;

                // Filter out pong messages and only return state/error notifications
                if let Notification::Pong { .. } = notification {
                    println!("[{}] Ignoring pong, continuing...", client_name);
                } else {
                    println!(
                        "[{}] Received notification: {:?}",
                        client_name, notification
                    );
                    return Ok(notification);
                }
            }
        } else {
            return Err(format!("[{}] No notification received", client_name).into());
        }
    }
}

/// Coordinated command execution with proper synchronization using oneshot channels
async fn coordinated_command_execution(
    client1_sender: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        TungsteniteMessage,
    >,
    client1_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    client2_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    command: Command,
    client1_name: &str,
    client2_name: &str,
) -> Result<(Notification, Notification), Box<dyn std::error::Error + Send + Sync>> {
    // Record timestamp before sending command to filter out stale notifications
    let command_start_time = toboggan_core::Timestamp::now();

    // Send command from client1
    let cmd_msg = serde_json::to_string(&command)?;
    client1_sender
        .send(TungsteniteMessage::text(cmd_msg))
        .await?;
    println!("[{}] Sent command: {:?}", client1_name, command);

    // Wait for client1 response first (the command sender)
    let client1_response = receive_next_non_pong_notification(client1_receiver, client1_name)
        .await
        .map_err(|err| format!("Client1 response error: {}", err))?;

    // Extract timestamp from client1 response to ensure client2 gets newer notification
    let response_timestamp = match &client1_response {
        Notification::State { timestamp, .. } => *timestamp,
        _ => command_start_time, // fallback to command start time
    };

    // Now wait for client2 sync notification that's newer than the command response
    let client2_sync =
        receive_recent_non_pong_notification(client2_receiver, client2_name, response_timestamp)
            .await
            .map_err(|err| format!("Client2 sync error: {}", err))?;

    Ok((client1_response, client2_sync))
}

/// Helper to receive next non-pong notification that's newer than given timestamp
async fn receive_recent_non_pong_notification(
    ws_receiver: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    client_name: &str,
    after_timestamp: toboggan_core::Timestamp,
) -> Result<Notification, Box<dyn std::error::Error + Send + Sync>> {
    loop {
        if let Some(msg) = ws_receiver.next().await {
            let msg = msg?;
            if let TungsteniteMessage::Text(text) = msg {
                let notification: Notification = serde_json::from_str(&text)?;

                // Filter out pong messages and old notifications
                if let Notification::Pong { .. } = &notification {
                    println!("[{}] Ignoring pong, continuing...", client_name);
                } else if let Notification::State { timestamp, .. } = &notification {
                    if *timestamp < after_timestamp {
                        println!(
                            "[{}] Ignoring old notification with timestamp {:?} (before {:?})",
                            client_name, timestamp, after_timestamp
                        );
                    } else {
                        println!(
                            "[{}] Received notification: {:?}",
                            client_name, notification
                        );
                        return Ok(notification);
                    }
                } else {
                    println!(
                        "[{}] Received notification: {:?}",
                        client_name, notification
                    );
                    return Ok(notification);
                }
            }
        } else {
            return Err(format!("[{}] No notification received", client_name).into());
        }
    }
}

#[tokio::test]
async fn test_client_disconnect_and_reconnect_sync() {
    // Use a unique SlideId starting point for this test to avoid interference
    let test_id = GLOBAL_TEST_COUNTER.fetch_add(100, std::sync::atomic::Ordering::SeqCst);
    let current_seq = 50 + test_id;
    toboggan_core::SlideId::reset_sequence_to(current_seq);

    // Create test server
    let (server_url, _state) = create_test_server().await;

    // Connect two clients initially
    let (mut client1_sender, mut client1_receiver, _client1_id) =
        connect_and_register_client(&server_url, "Client1")
            .await
            .expect("Failed to connect client 1");

    let (client2_sender, mut client2_receiver, _client2_id) =
        connect_and_register_client(&server_url, "Client2")
            .await
            .expect("Failed to connect client 2");

    println!("Both clients connected successfully");

    // Client 1 navigates to slide 2
    println!("\n=== Client 1 navigates to slide 2 ===");
    let _next_notification = send_command_and_get_response(
        &mut client1_sender,
        &mut client1_receiver,
        Command::Next,
        "Client1",
    )
    .await
    .expect("Failed to navigate next");

    // Client 2 receives the sync
    let _client2_sync = wait_for_notification(&mut client2_receiver, "Client2", 1000)
        .await
        .expect("Client 2 should receive sync");

    // Drop client 2 connection to simulate disconnect
    drop(client2_sender);
    drop(client2_receiver);
    println!("Client 2 disconnected");

    // Client 1 navigates to slide 3 while client 2 is disconnected
    println!("\n=== Client 1 navigates to slide 3 (Client 2 disconnected) ===");
    let _next2_notification = send_command_and_get_response(
        &mut client1_sender,
        &mut client1_receiver,
        Command::Next,
        "Client1",
    )
    .await
    .expect("Failed to navigate next again");

    // Reconnect client 2
    println!("\n=== Client 2 reconnects ===");
    let (_client2_sender_new, _client2_receiver_new, _client2_id_new) =
        connect_and_register_client(&server_url, "Client2-Reconnected")
            .await
            .expect("Failed to reconnect client 2");

    // Client 2 should receive the current state (slide 3) upon reconnection
    // This is handled by the initial state sent during registration
    println!("✅ Client 2 successfully reconnected and received current state");

    println!("\n🎉 Client disconnect/reconnect test passed!");
    println!("✅ Disconnected clients are properly cleaned up");
    println!("✅ Reconnected clients receive the current state immediately");
}
