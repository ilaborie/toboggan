use std::sync::Arc;

use pyo3::{exceptions::PyConnectionError, prelude::*};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::try_join;

use toboggan_client::{CommunicationMessage, TobogganApi, TobogganConfig, WebSocketClient};
use toboggan_core::{ClientConfig, Command, SlidesResponse, State as TState, TalkResponse};

/// Toboggan for Python
#[pymodule]
fn toboggan_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Talk>()?;
    m.add_class::<Slides>()?;
    m.add_class::<State>()?;
    m.add_class::<Toboggan>()?;

    Ok(())
}

/// Presentation metadata.
#[pyclass]
pub struct Talk(TalkResponse);

#[pymethods]
impl Talk {
    fn __repr__(&self) -> String {
        let title = &self.0.title;
        let date = &self.0.date;
        let slide_count = self.0.titles.len();
        let footer = self
            .0
            .footer
            .as_ref()
            .map_or(String::new(), |f| format!("\n  footer: {f}"));
        format!("Talk(\"{title}\", {date}, {slide_count} slides){footer}")
    }

    fn __str__(&self) -> String {
        self.0.title.clone()
    }

    /// The presentation title.
    #[getter]
    fn title(&self) -> &str {
        &self.0.title
    }

    /// The presentation date.
    #[getter]
    fn date(&self) -> String {
        self.0.date.to_string()
    }

    /// The optional footer text.
    #[getter]
    fn footer(&self) -> Option<&str> {
        self.0.footer.as_deref()
    }

    /// The slide titles.
    #[getter]
    fn titles(&self) -> Vec<String> {
        self.0.titles.clone()
    }
}

/// Collection of slides in the presentation.
#[pyclass]
pub struct Slides(SlidesResponse);

#[pymethods]
impl Slides {
    fn __repr__(&self) -> String {
        let count = self.0.slides.len();
        let slides = self
            .0
            .slides
            .iter()
            .enumerate()
            .map(|(i, slide)| format!("  {}: {slide}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Slides({count}):\n{slides}")
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        self.0.slides.len()
    }
}

/// Current presentation state.
#[pyclass]
pub struct State(TState);

#[pymethods]
impl State {
    fn __repr__(&self) -> String {
        match &self.0 {
            TState::Init => "State(Init)".to_string(),
            TState::Running {
                current,
                current_step,
            } => format!("State(Running, slide: {current}, step: {current_step})"),
            TState::Done {
                current,
                current_step,
            } => format!("State(Done, slide: {current}, step: {current_step})"),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Whether the presentation is in the initial state.
    #[getter]
    fn is_init(&self) -> bool {
        matches!(self.0, TState::Init)
    }

    /// Whether the presentation is currently running.
    #[getter]
    fn is_running(&self) -> bool {
        matches!(self.0, TState::Running { .. })
    }

    /// Whether the presentation is finished.
    #[getter]
    fn is_done(&self) -> bool {
        matches!(self.0, TState::Done { .. })
    }

    /// The current slide number (1-indexed), or None if not started.
    #[getter]
    fn slide(&self) -> Option<usize> {
        match &self.0 {
            TState::Init => None,
            TState::Running { current, .. } | TState::Done { current, .. } => {
                Some(current.index() + 1)
            }
        }
    }

    /// The current step within the slide, or None if not started.
    #[getter]
    fn step(&self) -> Option<usize> {
        match &self.0 {
            TState::Init => None,
            TState::Running { current_step, .. } | TState::Done { current_step, .. } => {
                Some(*current_step)
            }
        }
    }
}

/// Toboggan presentation client.
#[pyclass]
struct Toboggan {
    config: TobogganConfig,
    rt: Runtime,
    _ws: WebSocketClient,
    tx: UnboundedSender<Command>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    state: Arc<RwLock<TState>>,
}

impl Toboggan {
    fn send(&self, command: Command) {
        if let Err(err) = self.tx.send(command) {
            eprintln!("🚨 Oops, fail to send: {err}");
        }
    }
}

#[pymethods]
impl Toboggan {
    /// Creates a new Toboggan client and connects to the server.
    #[new]
    #[pyo3(signature = (host = "localhost", port = 8080))]
    pub fn __new__(host: &str, port: u16) -> PyResult<Self> {
        let config = TobogganConfig::new(host, port);

        let api_url = config.api_url();
        let api = TobogganApi::new(api_url);

        let ws_config = config.websocket();
        let (tx, rx) = mpsc::unbounded_channel();
        let (mut ws, rx_msg) = WebSocketClient::new(tx.clone(), rx, "Python", ws_config);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let state = Arc::<RwLock<TState>>::default();
        let talk = Arc::<RwLock<TalkResponse>>::default();
        let slides = Arc::<RwLock<SlidesResponse>>::default();

        let (initial_talk, initial_slides) = rt
            .block_on(async {
                let _read_messages = tokio::spawn(handle_state(
                    Arc::clone(&state),
                    Arc::clone(&talk),
                    Arc::clone(&slides),
                    api.clone(),
                    rx_msg,
                ));
                ws.connect().await;
                try_join!(api.talk(), api.slides())
            })
            .map_err(|err| PyConnectionError::new_err(err.to_string()))?;

        // Initialize talk and slides
        rt.block_on(async {
            *talk.write().await = initial_talk;
            *slides.write().await = initial_slides;
        });

        Ok(Self {
            rt,
            config,
            _ws: ws,
            tx,
            talk,
            slides,
            state,
        })
    }

    /// Gets the presentation metadata.
    #[getter]
    pub fn talk(&self) -> PyResult<Talk> {
        let talk = Arc::clone(&self.talk);
        let talk = self.rt.block_on(async {
            let guard = talk.read().await;
            TalkResponse::clone(&guard)
        });
        Ok(Talk(talk))
    }

    /// Gets all slides in the presentation.
    #[getter]
    pub fn slides(&self) -> PyResult<Slides> {
        let slides = Arc::clone(&self.slides);
        let slides = self.rt.block_on(async {
            let guard = slides.read().await;
            SlidesResponse::clone(&guard)
        });
        Ok(Slides(slides))
    }

    /// Gets the current presentation state.
    #[getter]
    pub fn state(&self) -> PyResult<State> {
        let state = Arc::clone(&self.state);
        let state = self.rt.block_on(async {
            let guard = state.read().await;
            TState::clone(&guard)
        });

        Ok(State(state))
    }

    /// Navigates to the previous slide.
    pub fn previous(&self) {
        self.send(Command::PreviousSlide);
    }

    /// Navigates to the next slide.
    pub fn next(&self) {
        self.send(Command::NextSlide);
    }

    pub fn __repr__(&self) -> String {
        format!("Toboggan({:?})", self.config)
    }

    pub fn __str__(&self) -> String {
        format!("Toboggan({})", self.config.api_url())
    }
}

async fn handle_state(
    state: Arc<RwLock<TState>>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    api: TobogganApi,
    mut rx: UnboundedReceiver<CommunicationMessage>,
) {
    println!(">>> Start listening incoming messages");
    while let Some(msg) = rx.recv().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                println!("📡 {status}");
            }
            CommunicationMessage::StateChange { state: new_state } => {
                let mut st = state.write().await;
                *st = new_state;
            }
            CommunicationMessage::TalkChange { state: new_state } => {
                println!("📝 Presentation updated - refetching talk and slides");

                // Refetch talk and slides from server
                match try_join!(api.talk(), api.slides()) {
                    Ok((new_talk, new_slides)) => {
                        // Update talk and slides atomically
                        {
                            let mut talk_guard = talk.write().await;
                            *talk_guard = new_talk;
                        }
                        {
                            let mut slides_guard = slides.write().await;
                            *slides_guard = new_slides;
                        }
                        // Update state after data is refreshed
                        {
                            let mut st = state.write().await;
                            *st = new_state;
                        }
                        println!("✅ Talk and slides updated successfully");
                    }
                    Err(err) => {
                        eprintln!("🚨 Failed to refetch talk and slides: {err}");
                        // Still update state even if refetch failed
                        let mut st = state.write().await;
                        *st = new_state;
                    }
                }
            }
            CommunicationMessage::Error { error } => {
                eprintln!("🚨 Oops: {error}");
            }
            CommunicationMessage::Registered { client_id } => {
                println!("🆔 Registered with id: {client_id:?}");
            }
            CommunicationMessage::ClientConnected { client_id, name } => {
                println!("👤 Client connected: {name} ({client_id:?})");
            }
            CommunicationMessage::ClientDisconnected { client_id, name } => {
                println!("👋 Client disconnected: {name} ({client_id:?})");
            }
        }
    }
    println!("<<< End listening incoming messages");
}
