use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, FromRef, State};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use toboggan_core::timeouts::HEARTBEAT_INTERVAL;
use toboggan_core::{ClientId, ClientRole, Command, Notification, Secret};
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

use crate::TobogganState;
use crate::services::{ClientService, TalkService};

pub(super) async fn websocket_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<TobogganState>,
) -> Response {
    let ip_addr = addr.ip();
    ws.on_upgrade(move |socket| handle_websocket(socket, state, ip_addr))
}

async fn handle_websocket(socket: WebSocket, state: TobogganState, ip_addr: IpAddr) {
    info!(%ip_addr, "New WebSocket connection established, waiting for Register command");

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let client_service = ClientService::from_ref(&state);

    // Wait for Register command from client
    let (client_id, client_name, client_role, notification_rx) = loop {
        match ws_receiver.next().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<Command>(&text) {
                Ok(Command::Register { name, token }) => {
                    // The handshake is where the role is settled, once: the
                    // socket's peer address cannot change under it, so there is
                    // no need to re-derive it per command.
                    let role = state.role_for(ip_addr, token.as_ref().map(Secret::expose));
                    let initial_notification = TalkService::from_ref(&state)
                        .create_initial_notification()
                        .await;
                    match client_service
                        .register_client(name.clone(), ip_addr, role, initial_notification)
                        .await
                    {
                        Ok((id, rx)) => {
                            // Send Registered notification to this client
                            let registered = Notification::registered(id, role);
                            if let Ok(msg) = serde_json::to_string(&registered)
                                && ws_sender.send(Message::Text(msg.into())).await.is_err()
                            {
                                error!("Failed to send Registered notification");
                                return;
                            }
                            break (id, name, role, rx);
                        }
                        Err(err) => {
                            error!("Failed to register client: {err}");
                            let error_notification = Notification::error(err.to_string());
                            if let Ok(msg) = serde_json::to_string(&error_notification) {
                                let _ = ws_sender.send(Message::Text(msg.into())).await;
                            }
                            return;
                        }
                    }
                }
                Ok(_) => {
                    // Ignore other commands until registered
                    warn!("Received command before registration, ignoring");
                }
                Err(err) => {
                    warn!(?err, "Failed to parse command");
                }
            },
            Some(Ok(Message::Close(_))) | None => {
                info!("WebSocket closed before registration");
                return;
            }
            _ => {}
        }
    };

    info!(?client_id, %client_name, %ip_addr, ?client_role, "Client registered via WebSocket");

    // Send initial state after registration
    if let Err(()) = send_initial_state(&mut ws_sender, &state, client_id).await {
        client_service.unregister_client(client_id).await;
        return;
    }

    let (notification_tx, notification_rx_internal) = mpsc::unbounded_channel::<Notification>();
    let error_notification_tx = notification_tx.clone();

    let watcher_task =
        spawn_notification_watcher_task(notification_rx, notification_tx.clone(), client_id);
    let sender_task =
        spawn_notification_sender_task(notification_rx_internal, ws_sender, client_id);
    let receiver_task = spawn_message_receiver_task(
        ws_receiver,
        state.clone(),
        error_notification_tx,
        client_id,
        client_role,
    );
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

    client_service.unregister_client(client_id).await;
    info!(
        ?client_id,
        "Client unregistered and WebSocket connection closed"
    );
}

/// Whether this socket's role bars it from this command.
///
/// The whole of the WebSocket's authorization, which is a different mechanism
/// from the [`Presenter`](super::presenter::Presenter) extractor the HTTP routes
/// use: the role here is settled once, at the `Register` frame, because the
/// socket outlives the request. Named and separate so it can be tested — it was
/// an inline condition, and `/api/ws` is what every browser client actually
/// drives the deck through, so this was the least covered branch in the crate.
///
/// An audience client is not disconnected for trying. A stale tab, or a
/// reconnect from a laptop that moved off the presenter's machine, is a mistake
/// rather than an attack: it is told, and ignored.
const fn refuses(command: &Command, role: ClientRole) -> bool {
    command.drives_the_deck() && !role.is_presenter()
}

async fn send_initial_state(
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    state: &TobogganState,
    client_id: ClientId,
) -> Result<(), ()> {
    let initial_notification = {
        let current_state = TalkService::from_ref(state).current_state().await;
        Notification::state(current_state)
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
    mut notification_rx: watch::Receiver<Notification>,
    notification_tx: mpsc::UnboundedSender<Notification>,
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
    mut notification_rx_internal: mpsc::UnboundedReceiver<Notification>,
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
    error_notification_tx: mpsc::UnboundedSender<Notification>,
    client_id: ClientId,
    client_role: ClientRole,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // The frame text is deliberately not logged: a `Register`
                    // carries the presenter token, and this ran at INFO for
                    // every message. The parsed command below is logged
                    // instead, and `Secret`'s `Debug` redacts it there.
                    match serde_json::from_str::<Command>(&text) {
                        Ok(command) => {
                            info!(?client_id, ?command, "Processing command");

                            if let Command::Unregister { client } = &command
                                && *client == client_id
                            {
                                info!(
                                    ?client_id,
                                    "Client unregistering itself, closing connection"
                                );
                                break;
                            }

                            if refuses(&command, client_role) {
                                warn!(?client_id, ?command, "Refused a command from the audience");
                                let refusal =
                                    Notification::error("This client is watching, not presenting");
                                let _ = error_notification_tx.send(refusal);
                                continue;
                            }

                            let _notification = state.handle_command(&command).await;
                        }
                        Err(err) => {
                            // Not the frame text: a `Register` that failed to
                            // parse still carries the token that was in it.
                            warn!(
                                ?client_id,
                                ?err,
                                "Failed to parse command from WebSocket message"
                            );

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
                Ok(Message::Ping(_) | Message::Pong(_)) => {}
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
    notification_tx: mpsc::UnboundedSender<Notification>,
    client_id: ClientId,
    heartbeat_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(heartbeat_interval);

        loop {
            interval.tick().await;

            let ping_notification = Notification::PONG;
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

#[cfg(test)]
mod tests {
    use toboggan_core::SlideId;

    use super::*;

    /// The audience may watch and nothing else. `drives_the_deck` is a negation
    /// of the harmless set, so a command added later is refused here by default
    /// rather than slipping through.
    #[test]
    fn the_audience_is_refused_everything_that_moves_the_deck() {
        for command in [
            Command::First,
            Command::Last,
            Command::NextSlide,
            Command::PreviousSlide,
            Command::NextStep,
            Command::PreviousStep,
            Command::Blink,
            Command::GoTo {
                slide: SlideId::FIRST,
            },
        ] {
            assert!(
                refuses(&command, ClientRole::Audience),
                "audience must not send {command:?}"
            );
            assert!(
                !refuses(&command, ClientRole::Presenter),
                "presenter must be able to send {command:?}"
            );
        }
    }

    /// Registering and the heartbeat are how a client becomes an audience
    /// member at all, so refusing them would refuse the connection itself.
    #[test]
    fn the_audience_may_still_register_and_ping() {
        for command in [
            Command::Register {
                name: "watcher".to_owned(),
                token: None,
            },
            Command::Unregister {
                client: ClientId::from_key(slotmap::DefaultKey::default()),
            },
            Command::Ping,
        ] {
            assert!(
                !refuses(&command, ClientRole::Audience),
                "audience must be able to send {command:?}"
            );
        }
    }
}
