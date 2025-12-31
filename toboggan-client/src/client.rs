use std::sync::Arc;

use toboggan_core::{Command, Slide, State, TalkResponse};
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

use crate::{
    CommunicationMessage, NotificationHandler, TobogganApi, TobogganApiError,
    TobogganWebsocketConfig, WebSocketClient,
};

/// Core client for connecting to a Toboggan presentation server.
///
/// This struct manages:
/// - WebSocket connection via `WebSocketClient`
/// - Shared state for talk, slides, and presentation state
/// - Message dispatch loop calling `NotificationHandler` methods
/// - `TalkChange` refetch logic
/// - Command sending via channel
///
/// Generic over `H: NotificationHandler` to allow different implementations
/// for TUI, Desktop, Mobile, etc.
pub struct TobogganClientCore<H: NotificationHandler> {
    // Watch channels for state (sender for writing, receiver for reading)
    talk_tx: watch::Sender<Option<TalkResponse>>,
    slides_tx: watch::Sender<Arc<[Slide]>>,
    state_tx: watch::Sender<Option<State>>,

    talk_rx: watch::Receiver<Option<TalkResponse>>,
    slides_rx: watch::Receiver<Arc<[Slide]>>,
    state_rx: watch::Receiver<Option<State>>,

    // Handler for notifications
    handler: Arc<H>,

    // Connection management
    api: TobogganApi,
    tx_cmd: Option<mpsc::UnboundedSender<Command>>,
    ws_config: TobogganWebsocketConfig,
    client_name: String,
}

impl<H: NotificationHandler + 'static> TobogganClientCore<H> {
    /// Create a new client core.
    ///
    /// # Arguments
    /// - `api_url`: Base URL for the HTTP API (e.g., `http://localhost:8080`)
    /// - `websocket_config`: Configuration for WebSocket connection
    /// - `client_name`: Name to register with the server
    /// - `handler`: Handler for notifications
    #[must_use]
    pub fn new(
        api_url: &str,
        websocket_config: TobogganWebsocketConfig,
        client_name: impl Into<String>,
        handler: H,
    ) -> Self {
        let (talk_tx, talk_rx) = watch::channel(None);
        let (slides_tx, slides_rx) = watch::channel::<Arc<[Slide]>>(Arc::from([]));
        let (state_tx, state_rx) = watch::channel(None);

        Self::new_internal(
            api_url,
            websocket_config,
            client_name,
            handler,
            talk_tx,
            talk_rx,
            slides_tx,
            slides_rx,
            state_tx,
            state_rx,
        )
    }

    /// Create a new client core with external slides channel.
    ///
    /// This variant is useful when the handler needs a slides receiver
    /// (e.g., `NotificationAdapter` in mobile).
    #[must_use]
    pub fn new_with_slides_channel(
        api_url: &str,
        websocket_config: TobogganWebsocketConfig,
        client_name: impl Into<String>,
        handler: H,
        slides_tx: watch::Sender<Arc<[Slide]>>,
        slides_rx: watch::Receiver<Arc<[Slide]>>,
    ) -> Self {
        let (talk_tx, talk_rx) = watch::channel(None);
        let (state_tx, state_rx) = watch::channel(None);

        Self::new_internal(
            api_url,
            websocket_config,
            client_name,
            handler,
            talk_tx,
            talk_rx,
            slides_tx,
            slides_rx,
            state_tx,
            state_rx,
        )
    }

    /// Create a new client core with external slides and talk channels.
    ///
    /// This variant is useful when the handler needs access to both slides
    /// and talk data (e.g., for step counts in mobile).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_external_channels(
        api_url: &str,
        websocket_config: TobogganWebsocketConfig,
        client_name: impl Into<String>,
        handler: H,
        slides_tx: watch::Sender<Arc<[Slide]>>,
        slides_rx: watch::Receiver<Arc<[Slide]>>,
        talk_tx: watch::Sender<Option<TalkResponse>>,
        talk_rx: watch::Receiver<Option<TalkResponse>>,
    ) -> Self {
        let (state_tx, state_rx) = watch::channel(None);

        Self::new_internal(
            api_url,
            websocket_config,
            client_name,
            handler,
            talk_tx,
            talk_rx,
            slides_tx,
            slides_rx,
            state_tx,
            state_rx,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_internal(
        api_url: &str,
        websocket_config: TobogganWebsocketConfig,
        client_name: impl Into<String>,
        handler: H,
        talk_tx: watch::Sender<Option<TalkResponse>>,
        talk_rx: watch::Receiver<Option<TalkResponse>>,
        slides_tx: watch::Sender<Arc<[Slide]>>,
        slides_rx: watch::Receiver<Arc<[Slide]>>,
        state_tx: watch::Sender<Option<State>>,
        state_rx: watch::Receiver<Option<State>>,
    ) -> Self {
        let api = TobogganApi::new(api_url);

        Self {
            talk_tx,
            slides_tx,
            state_tx,
            talk_rx,
            slides_rx,
            state_rx,
            handler: Arc::new(handler),
            api,
            tx_cmd: None,
            ws_config: websocket_config,
            client_name: client_name.into(),
        }
    }

    /// Get the API client for making HTTP requests.
    #[must_use]
    pub fn api(&self) -> &TobogganApi {
        &self.api
    }

    /// Connect to the server and start handling messages.
    ///
    /// This will:
    /// 1. Fetch initial talk and slides from the HTTP API
    /// 2. Establish WebSocket connection
    /// 3. Spawn a task to handle incoming messages
    pub async fn connect(&mut self) {
        // Load initial talk
        match self.api.talk().await {
            Ok(talk) => {
                info!("Loaded talk: {}", talk.title);
                let _ = self.talk_tx.send(Some(talk));
            }
            Err(err) => {
                error!("Failed to load talk: {err}");
                self.handler.on_error(format!("Failed to load talk: {err}"));
            }
        }

        // Load initial slides
        match self.api.slides().await {
            Ok(slides_response) => {
                info!("Loaded {} slides", slides_response.slides.len());
                let _ = self.slides_tx.send(Arc::from(slides_response.slides));
            }
            Err(err) => {
                error!("Failed to load slides: {err}");
                self.handler
                    .on_error(format!("Failed to load slides: {err}"));
            }
        }

        // Create channels for WebSocket communication
        let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();
        self.tx_cmd = Some(tx_cmd.clone());

        // Create WebSocket client
        let (mut ws_client, mut rx_msg) = WebSocketClient::new(
            tx_cmd,
            rx_cmd,
            self.client_name.clone(),
            self.ws_config.clone(),
        );

        // Connect WebSocket in background
        tokio::spawn(async move {
            ws_client.connect().await;
        });

        // Spawn message handling task
        let handler = Arc::clone(&self.handler);
        let api = self.api.clone();
        let talk_tx = self.talk_tx.clone();
        let slides_tx = self.slides_tx.clone();
        let state_tx = self.state_tx.clone();

        tokio::spawn(async move {
            Self::handle_incoming_messages(handler, api, talk_tx, slides_tx, state_tx, &mut rx_msg)
                .await;
        });
    }

    /// Send a command to the server.
    pub fn send_command(&self, command: Command) {
        if let Some(tx_cmd) = &self.tx_cmd {
            if let Err(err) = tx_cmd.send(command) {
                error!("Failed to send command: {err}");
            }
        } else {
            error!("WebSocket command channel not available");
        }
    }

    /// Get the current talk metadata.
    #[must_use]
    pub fn get_talk(&self) -> Option<TalkResponse> {
        self.talk_rx.borrow().clone()
    }

    /// Get the current slides.
    #[must_use]
    pub fn get_slides(&self) -> Vec<Slide> {
        self.slides_rx.borrow().to_vec()
    }

    /// Get the current presentation state.
    #[must_use]
    pub fn get_state(&self) -> Option<State> {
        self.state_rx.borrow().clone()
    }

    /// Get a clone of the slides receiver for external consumers.
    ///
    /// This is useful for components that need to observe slides changes
    /// (e.g., `NotificationAdapter` in mobile).
    #[must_use]
    pub fn slides_receiver(&self) -> watch::Receiver<Arc<[Slide]>> {
        self.slides_rx.clone()
    }

    /// Internal message handling loop.
    async fn handle_incoming_messages(
        handler: Arc<H>,
        api: TobogganApi,
        talk_tx: watch::Sender<Option<TalkResponse>>,
        slides_tx: watch::Sender<Arc<[Slide]>>,
        state_tx: watch::Sender<Option<State>>,
        rx: &mut mpsc::UnboundedReceiver<CommunicationMessage>,
    ) {
        while let Some(msg) = rx.recv().await {
            match msg {
                CommunicationMessage::ConnectionStatusChange { status } => {
                    handler.on_connection_status_change(status);
                }
                CommunicationMessage::StateChange { state: new_state } => {
                    let _ = state_tx.send(Some(new_state.clone()));
                    handler.on_state_change(new_state);
                }
                CommunicationMessage::TalkChange { state: new_state } => {
                    info!("Presentation updated - refetching talk and slides");

                    // Refetch talk and slides from server
                    match refetch_talk_and_slides(&api).await {
                        Ok((new_talk, new_slides)) => {
                            info!("Talk and slides refetched successfully");
                            let _ = talk_tx.send(Some(new_talk));
                            let _ = slides_tx.send(Arc::from(new_slides));
                        }
                        Err(err) => {
                            error!("Failed to refetch talk and slides: {err}");
                        }
                    }

                    // Update state
                    let _ = state_tx.send(Some(new_state.clone()));

                    handler.on_talk_change(new_state);
                }
                CommunicationMessage::Error { error } => {
                    handler.on_error(error);
                }
                CommunicationMessage::Registered { client_id } => {
                    handler.on_registered(client_id);
                }
                CommunicationMessage::ClientConnected { client_id, name } => {
                    handler.on_client_connected(client_id, name);
                }
                CommunicationMessage::ClientDisconnected { client_id, name } => {
                    handler.on_client_disconnected(client_id, name);
                }
            }
        }
    }
}

/// Utility function to refetch talk and slides from the server.
///
/// This is useful for clients that want to trigger a refetch manually.
pub async fn refetch_talk_and_slides(
    api: &TobogganApi,
) -> Result<(TalkResponse, Vec<Slide>), TobogganApiError> {
    let (talk, slides_response) = tokio::try_join!(api.talk(), api.slides())?;
    Ok((talk, slides_response.slides))
}
