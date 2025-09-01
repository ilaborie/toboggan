use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use toboggan_core::{ClientId, Command, Notification};
use tracing::{error, info, warn};

use crate::TobogganState;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<TobogganState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: TobogganState) {
    let client_id = ClientId::new();
    info!(?client_id, "New WebSocket connection established");

    let notification_rx = match state.register_client(client_id).await {
        Ok(rx) => rx,
        Err(err) => {
            error!(?client_id, "Failed to register client: {err}");
            return;
        }
    };

    let (mut ws_sender, ws_receiver) = socket.split();

    if let Err(()) = send_initial_state(&mut ws_sender, &state, client_id).await {
        return;
    }

    let (notification_tx, notification_rx_internal) =
        tokio::sync::mpsc::unbounded_channel::<Notification>();
    let error_notification_tx = notification_tx.clone();

    let watcher_task =
        spawn_notification_watcher_task(notification_rx, notification_tx.clone(), client_id);
    let sender_task =
        spawn_notification_sender_task(notification_rx_internal, ws_sender, client_id);
    let receiver_task =
        spawn_message_receiver_task(ws_receiver, state.clone(), error_notification_tx, client_id);
    let heartbeat_task = spawn_heartbeat_task(notification_tx, client_id, HEARTBEAT_INTERVAL);

    tokio::select! {
        _ = watcher_task => {
            info!(?client_id, "Watcher task completed");
        }
        _ = sender_task => {
            info!(?client_id, "Sender task completed");
        }
        _ = receiver_task => {
            info!(?client_id, "Receiver task completed");
        }
        _ = heartbeat_task => {
            info!(?client_id, "Heartbeat task completed");
        }
    }

    state.unregister_client(client_id);
    info!(
        ?client_id,
        "Client unregistered and WebSocket connection closed"
    );
}

async fn send_initial_state(
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    state: &TobogganState,
    client_id: ClientId,
) -> Result<(), ()> {
    let initial_notification = {
        let current_state = state.current_state().await;
        Notification::state(current_state.clone())
    };

    if let Ok(msg) = serde_json::to_string(&initial_notification)
        && let Err(err) = ws_sender.send(Message::Text(msg.into())).await
    {
        error!(?client_id, ?err, "Failed to send initial state to client");
        return Err(());
    }

    Ok(())
}

fn spawn_notification_watcher_task(
    mut notification_rx: tokio::sync::watch::Receiver<Notification>,
    notification_tx: tokio::sync::mpsc::UnboundedSender<Notification>,
    client_id: ClientId,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while notification_rx.changed().await.is_ok() {
            let notification = notification_rx.borrow().clone();
            if notification_tx.send(notification).is_err() {
                warn!(
                    ?client_id,
                    "Failed to send notification to internal channel, receiver may be closed"
                );
                break;
            }
        }
        info!(?client_id, "Notification watcher task finished");
    })
}

fn spawn_notification_sender_task(
    mut notification_rx_internal: tokio::sync::mpsc::UnboundedReceiver<Notification>,
    mut ws_sender: futures::stream::SplitSink<WebSocket, Message>,
    client_id: ClientId,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(notification) = notification_rx_internal.recv().await {
            match serde_json::to_string(&notification) {
                Ok(msg) => {
                    if let Err(err) = ws_sender.send(Message::Text(msg.into())).await {
                        warn!(
                            ?client_id,
                            ?err,
                            "Failed to send notification to client, connection may be closed"
                        );
                        break;
                    }
                }
                Err(err) => {
                    error!(?client_id, ?err, "Failed to serialize notification");
                }
            }
        }
        info!(?client_id, "Notification sender task finished");
    })
}

fn spawn_message_receiver_task(
    mut ws_receiver: futures::stream::SplitStream<WebSocket>,
    state: TobogganState,
    error_notification_tx: tokio::sync::mpsc::UnboundedSender<Notification>,
    client_id: ClientId,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!(?client_id, message = %text, "Received WebSocket message");

                    match serde_json::from_str::<Command>(&text) {
                        Ok(command) => {
                            info!(?client_id, ?command, "Processing command");

                            match &command {
                                Command::Register { client, .. } => {
                                    info!(?client_id, registered_client = ?client, "Client registration via WebSocket");
                                }
                                Command::Unregister { client } => {
                                    info!(?client_id, unregistered_client = ?client, "Client unregistration via WebSocket");
                                    if *client == client_id {
                                        info!(
                                            ?client_id,
                                            "Client unregistering itself, closing connection"
                                        );
                                        break;
                                    }
                                }
                                _ => {}
                            }

                            let _notification = state.handle_command(&command).await;
                        }
                        Err(err) => {
                            warn!(?client_id, ?err, message = %text, "Failed to parse command from WebSocket message");

                            let error_notification =
                                Notification::error(format!("Invalid command format: {err}"));
                            if error_notification_tx.send(error_notification).is_err() {
                                error!(
                                    ?client_id,
                                    "Failed to send error notification to internal channel"
                                );
                            }
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!(?client_id, "Received binary message, ignoring");
                }
                Ok(Message::Close(_)) => {
                    info!(?client_id, "WebSocket connection closed by client");
                    break;
                }
                Ok(Message::Ping(_)) => {
                    info!(
                        ?client_id,
                        "Received ping (pong will be sent automatically by axum)"
                    );
                }
                Ok(Message::Pong(_)) => {
                    info!(?client_id, "Received pong");
                }
                Err(err) => {
                    warn!(?client_id, ?err, "WebSocket error");
                    break;
                }
            }
        }
        info!(?client_id, "Message receiver task finished");
    })
}

fn spawn_heartbeat_task(
    notification_tx: tokio::sync::mpsc::UnboundedSender<Notification>,
    client_id: ClientId,
    heartbeat_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(heartbeat_interval);

        loop {
            interval.tick().await;

            let ping_notification = Notification::pong();
            if notification_tx.send(ping_notification).is_err() {
                info!(
                    ?client_id,
                    "Heartbeat task stopping - notification channel closed"
                );
                break;
            }
        }

        info!(?client_id, "Heartbeat task finished");
    })
}
