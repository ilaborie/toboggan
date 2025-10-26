use std::sync::Arc;

use pyo3::{exceptions::PyConnectionError, prelude::*};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::try_join;

use toboggan_client::{CommunicationMessage, TobogganApi, TobogganConfig, WebSocketClient};
use toboggan_core::{ClientConfig as _, Command, SlidesResponse, State as TState, TalkResponse};

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
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.0.title.clone()
    }
}

/// Collection of slides in the presentation.
#[pyclass]
pub struct Slides(SlidesResponse);

#[pymethods]
impl Slides {
    fn __str__(&self) -> String {
        let titles = self
            .0
            .slides
            .iter()
            .map(|slide| slide.to_string())
            .collect::<Vec<_>>();
        format!("{titles:?}")
    }
}

/// Current presentation state.
#[pyclass]
pub struct State(TState);

#[pymethods]
impl State {
    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// Toboggan presentation client.
#[pyclass]
struct Toboggan {
    config: TobogganConfig,
    rt: Runtime,
    _ws: WebSocketClient,
    tx: UnboundedSender<Command>,
    talk: TalkResponse,
    slides: SlidesResponse,
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

        let client_id = config.client_id();
        let ws_config = config.websocket();
        let (tx, rx) = mpsc::unbounded_channel();
        let (mut ws, rx_msg) = WebSocketClient::new(tx.clone(), rx, client_id, ws_config);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let state = Arc::<RwLock<TState>>::default();
        let (talk, slides) = rt
            .block_on(async {
                let _read_messages = tokio::spawn(handle_state(Arc::clone(&state), rx_msg));
                ws.connect().await;
                try_join!(api.talk(), api.slides())
            })
            .map_err(|err| PyConnectionError::new_err(err.to_string()))?;

        Ok(Self {
            rt: rt,
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
        Ok(Talk(self.talk.clone()))
    }

    /// Gets all slides in the presentation.
    #[getter]
    pub fn slides(&self) -> PyResult<Slides> {
        Ok(Slides(self.slides.clone()))
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
        self.send(Command::Previous);
    }

    /// Navigates to the next slide.
    pub fn next(&self) {
        self.send(Command::Next);
    }

    pub fn __repr__(&self) -> String {
        format!("Toboggan({:?})", self.config)
    }

    pub fn __str__(&self) -> String {
        format!("Toboggan({})", self.config.api_url())
    }
}

async fn handle_state(state: Arc<RwLock<TState>>, mut rx: UnboundedReceiver<CommunicationMessage>) {
    println!(">>> Start listenning incomming messages");
    while let Some(msg) = rx.recv().await {
        let mut st = state.write().await;
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                print!("📡 {status}");
            }
            CommunicationMessage::StateChange { state } => {
                *st = state;
            }
            CommunicationMessage::Error { error } => {
                eprintln!("🚨 Oops: {error}");
            }
        }
        println!("<<< End listenning incomming messages");
    }
}
