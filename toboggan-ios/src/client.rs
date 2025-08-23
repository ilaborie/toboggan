use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use toboggan_client::{
    CommunicationMessage, TobogganApi, TobogganWebsocketConfig, WebSocketClient,
};
use toboggan_core::{ClientId, Command as CoreCommand};
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, mpsc};

use super::{ClientNotificationHandler, Command, Slide, State, Talk};

/// The toboggan client
#[derive(Debug, Clone, uniffi::Record)]
pub struct ClientConfig {
    /// The server URL, like `http://localhost:8080`
    pub url: String,

    /// The maximum number of retry if the connection is not working
    pub max_retries: u32,

    /// The delay between retries
    pub retry_delay: Duration,
}

#[derive(uniffi::Object)]
pub struct TobogganClient {
    talk: Arc<Mutex<Option<Talk>>>,
    state: Arc<Mutex<Option<State>>>,
    slides: Arc<Mutex<Vec<Slide>>>,
    is_connected: Arc<AtomicBool>,

    handler: Arc<dyn ClientNotificationHandler>,

    api: TobogganApi,
    tx: mpsc::UnboundedSender<CoreCommand>,
    ws: Arc<Mutex<WebSocketClient>>,
    rx_msg: Arc<Mutex<mpsc::UnboundedReceiver<CommunicationMessage>>>,

    runtime: Runtime,
}

impl TobogganClient {
    async fn read_incoming_messages(
        handler: Arc<dyn ClientNotificationHandler>,
        total_slides: usize,
        shared_state: Arc<Mutex<Option<State>>>,
        is_connected: Arc<AtomicBool>,
        rx: &mut mpsc::UnboundedReceiver<CommunicationMessage>,
    ) {
        while let Some(msg) = rx.recv().await {
            println!("🦀  Receiving: {msg:?}");
            match msg {
                CommunicationMessage::ConnectionStatusChange { status } => {
                    let connected = matches!(status, toboggan_client::ConnectionStatus::Connected);
                    is_connected.store(connected, Ordering::Relaxed);
                    handler.on_connection_status_change(status.into());
                }
                CommunicationMessage::StateChange { state: new_state } => {
                    let state_value = State::new(total_slides, &new_state);
                    {
                        let mut state_guard = shared_state.lock().await;
                        *state_guard = Some(state_value.clone());
                    }
                    handler.on_state_change(state_value);
                }
                CommunicationMessage::Error { error } => {
                    handler.on_error(error);
                }
            }
        }
    }
}

#[uniffi::export]
impl TobogganClient {
    #[uniffi::constructor]
    pub fn new(config: ClientConfig, handler: Arc<dyn ClientNotificationHandler>) -> Self {
        println!("🦀 using {config:#?}");
        let ClientConfig {
            url,
            max_retries,
            retry_delay,
        } = config;
        let api = TobogganApi::new(url.trim_end_matches('/'));
        let client_id = ClientId::new();

        let websocket_url = if url.starts_with("http://") {
            format!("ws://{}/api/ws", url.trim_start_matches("http://"))
        } else if url.starts_with("https://") {
            format!("wss://{}/api/ws", url.trim_start_matches("https://"))
        } else {
            panic!("invalid url '{url}', expected 'http(s)://<host>:<port>'");
        };

        let websocket = TobogganWebsocketConfig {
            websocket_url,
            max_retries: max_retries as usize,
            retry_delay,
            max_retry_delay: retry_delay * max_retries,
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let (ws, rx_msg) = WebSocketClient::new(tx.clone(), rx, client_id, websocket);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("having a tokio runtime");

        Self {
            talk: Arc::default(),
            state: Arc::default(),
            slides: Arc::default(),
            is_connected: Arc::default(),
            handler,
            api,
            tx,
            ws: Arc::new(Mutex::new(ws)),
            rx_msg: Arc::new(Mutex::new(rx_msg)),
            runtime,
        }
    }

    pub fn connect(&self) {
        self.runtime.block_on({
            let api = self.api.clone();
            let shared_talk = Arc::clone(&self.talk);
            let shared_slides = Arc::clone(&self.slides);
            let shared_ws = Arc::clone(&self.ws);
            let handler = Arc::clone(&self.handler);
            let rx_msg = Arc::clone(&self.rx_msg);

            async move {
                // Loading talk
                {
                    let tk = api.talk().await.expect("find a talk");
                    println!("🦀  talk: {tk:#?}");
                    let mut talk: tokio::sync::MutexGuard<'_, Option<Talk>> =
                        shared_talk.lock().await;
                    *talk = Some(tk.into());
                }

                // Loading slides
                let total_slides = {
                    let slides = api.slides().await.expect("find a talk").slides;
                    println!("🦀  count slides: {}", slides.len());
                    let mut sld = shared_slides.lock().await;
                    for slide in slides {
                        sld.push(slide.into());
                    }
                    sld.len()
                };

                // Connect to WebSocket
                let mut ws = shared_ws.lock().await;
                ws.connect().await;
                println!("🦀  connected");

                // Reading incoming messages
                let state_for_messages = Arc::clone(&self.state);
                let is_connected_for_messages = Arc::clone(&self.is_connected);
                tokio::spawn(async move {
                    let mut rx = rx_msg.lock().await;
                    Self::read_incoming_messages(
                        handler,
                        total_slides,
                        state_for_messages,
                        is_connected_for_messages,
                        &mut rx,
                    )
                    .await;
                });
            }
        });
    }

    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub fn send_command(&self, command: Command) {
        if let Err(error) = self.tx.send(command.into()) {
            println!("🦀 Error sending command {command:?}: {error:#?}");
        }
    }

    #[must_use]
    pub fn get_state(&self) -> Option<State> {
        let state = Arc::clone(&self.state);
        self.runtime.block_on(async move {
            let st = state.lock().await;
            st.as_ref().cloned()
        })
    }

    #[must_use]
    pub fn get_slide(&self, index: u32) -> Option<Slide> {
        let slides = Arc::clone(&self.slides);
        self.runtime.block_on(async move {
            let st = slides.lock().await;
            st.get(index as usize).cloned()
        })
    }

    #[must_use]
    pub fn get_talk(&self) -> Option<Talk> {
        let talk = Arc::clone(&self.talk);
        self.runtime.block_on(async move {
            let st = talk.lock().await;
            st.as_ref().cloned()
        })
    }
}
