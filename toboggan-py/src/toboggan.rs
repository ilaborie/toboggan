use std::sync::Arc;
use std::time::Duration;

use pyo3::exceptions::{PyConnectionError, PyPermissionError, PyRuntimeError};
use pyo3::prelude::*;
use toboggan_client::{
    CommunicationMessage, TobogganApi, TobogganApiError, TobogganConfig, WebSocketClient,
};
use toboggan_core::{
    ClientConfig, ClientRole, Command, Notification, Secret, SlidesResponse, State as TState,
    TalkResponse, goto_command,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::{RwLock, watch};
use tokio::try_join;

use crate::client_info::role_name;
use crate::{ClientInfo, Slides, State, Talk};

/// Where the other Toboggan clients read their presenter token from.
///
/// `toboggan tui` and `toboggan desktop` take it through clap's `env`, so a
/// script run beside them should not have to be told separately.
const PRESENTER_TOKEN_ENV: &str = "TOBOGGAN_PRESENTER_TOKEN";

/// How long the constructor waits for the server to answer registration.
///
/// The role decides whether this client's commands will be obeyed at all, and
/// the answer comes on the socket rather than in the REST responses the
/// constructor already waits for. Bounded, because a server that never answers
/// must not leave `Toboggan(...)` hanging in a REPL — the role then reads as
/// `None`, which is honest.
const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(5);

/// Toboggan presentation client.
///
/// Navigation is synchronous: when [`Toboggan::next`] and its siblings return,
/// the server has applied the command and the [`Toboggan::state`] getter
/// reports where the deck now is. The socket is still there, doing the job only
/// it can do — reporting moves *other* clients made, and deck reloads.
#[pyclass]
pub struct Toboggan {
    config: TobogganConfig,
    rt: Runtime,
    _ws: WebSocketClient,
    api: TobogganApi,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    state: Arc<RwLock<TState>>,
    role: watch::Receiver<Option<ClientRole>>,
}

impl Toboggan {
    /// Sends a command and returns once the server has applied it.
    ///
    /// `POST /api/command` answers with the state the command produced, so the
    /// cache is right *before* this returns: the `state` getter can no longer
    /// hand back the position the deck was in before the call. That is the
    /// whole point — a command over the socket is a `send` with nothing to wait
    /// on, and a Python caller had no way to know when it had landed.
    ///
    /// A third party moving the deck at the same moment can have its pushed
    /// frame land either side of this write. The promise still holds — this
    /// command *was* applied, and what is cached is the state it produced — and
    /// the next frame settles any disagreement.
    fn drive(&self, py: Python<'_>, command: Command) -> PyResult<()> {
        let api = self.api.clone();
        let state = Arc::clone(&self.state);
        let handle = self.rt.handle().clone();

        // Detached: this is a network round trip, and a `block_on` that keeps
        // the GIL freezes every other Python thread for its whole duration.
        let notification = py
            .detach(move || {
                handle.block_on(async move {
                    let notification = api.command(command).await?;
                    if let Notification::State { state: applied }
                    | Notification::TalkChange { state: applied } = &notification
                    {
                        *state.write().await = applied.clone();
                    }
                    Ok::<_, TobogganApiError>(notification)
                })
            })
            .map_err(|err| refused_or_unreachable(&err))?;

        match notification {
            Notification::Error { message } => Err(PyRuntimeError::new_err(message)),
            _ => Ok(()),
        }
    }
}

#[pymethods]
impl Toboggan {
    /// Creates a new Toboggan client and connects to the server.
    ///
    /// # Errors
    /// Raises `ConnectionError` if the server cannot be reached or the deck
    /// cannot be fetched. A registration that goes unanswered is *not* an
    /// error: the deck is there, and `role` reports None until it arrives.
    #[new]
    #[pyo3(signature = (host = "localhost", port = 8080, presenter_token = None))]
    pub fn __new__(
        py: Python<'_>,
        host: &str,
        port: u16,
        presenter_token: Option<&str>,
    ) -> PyResult<Self> {
        let token = resolve_token(presenter_token);
        let config = TobogganConfig::new(host, port).with_presenter_token(token.clone());

        // The token goes on both halves: the socket offers it in `Register`,
        // and the REST side needs it for `/api/command` and `/api/clients`.
        let api = TobogganApi::new(config.api_url()).with_presenter_token(token);

        let ws_config = config.websocket();
        let (tx, rx) = mpsc::unbounded_channel();
        let (mut ws, rx_msg) = WebSocketClient::new(tx, rx, "Python", ws_config);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let state = Arc::<RwLock<TState>>::default();
        let talk = Arc::<RwLock<TalkResponse>>::default();
        let slides = Arc::<RwLock<SlidesResponse>>::default();
        let (role_tx, mut role_rx) = watch::channel(None);

        // Detached for the same reason as `drive`: connecting and fetching the
        // deck is network work, and the registration wait below can take
        // seconds. Holding the GIL across either freezes the interpreter — a
        // constructor is no better a place to do that than a method.
        let (initial_talk, initial_slides) = py
            .detach(|| {
                rt.block_on(async {
                    let _read_messages = tokio::spawn(handle_state(
                        Arc::clone(&state),
                        Arc::clone(&talk),
                        Arc::clone(&slides),
                        role_tx,
                        api.clone(),
                        rx_msg,
                    ));
                    ws.connect().await;
                    try_join!(api.talk(), api.slides())
                })
            })
            .map_err(|err| PyConnectionError::new_err(err.to_string()))?;

        py.detach(|| {
            rt.block_on(async {
                *talk.write().await = initial_talk;
                *slides.write().await = initial_slides;

                // A timeout here is not a failure: the deck was fetched, so the
                // client is usable — it just does not know yet what it is
                // allowed to do.
                let registered = role_rx.wait_for(Option::is_some);
                if tokio::time::timeout(REGISTRATION_TIMEOUT, registered)
                    .await
                    .is_err()
                {
                    eprintln!("⏳ The server has not answered registration; role unknown for now");
                }
            });
        });

        Ok(Self {
            rt,
            config,
            _ws: ws,
            api,
            talk,
            slides,
            state,
            role: role_rx,
        })
    }

    /// Gets the presentation metadata.
    #[getter]
    pub fn talk(&self) -> Talk {
        let talk = Arc::clone(&self.talk);
        let talk = self.rt.block_on(async {
            let guard = talk.read().await;
            TalkResponse::clone(&guard)
        });
        Talk(talk)
    }

    /// Gets all slides in the presentation.
    #[getter]
    pub fn slides(&self) -> Slides {
        let slides = Arc::clone(&self.slides);
        let slides = self.rt.block_on(async {
            let guard = slides.read().await;
            SlidesResponse::clone(&guard)
        });
        Slides(slides)
    }

    /// Where the deck is now.
    ///
    /// Trustworthy immediately after a navigation call: those return only once
    /// the server has applied the command and this cache holds its answer.
    #[getter]
    pub fn state(&self) -> State {
        let state = Arc::clone(&self.state);
        let state = self.rt.block_on(async {
            let guard = state.read().await;
            TState::clone(&guard)
        });

        State(state)
    }

    /// The role the server granted this connection: `"presenter"`,
    /// `"audience"`, or None while registration is still unanswered.
    ///
    /// A client never claims a role — it offers a token and the server decides.
    #[getter]
    pub fn role(&self) -> Option<&'static str> {
        (*self.role.borrow()).map(role_name)
    }

    /// Whether this client may drive the deck.
    ///
    /// False for a connection from another machine that offered no presenter
    /// token: the navigation methods below then raise `PermissionError`.
    #[getter]
    pub fn is_presenter(&self) -> bool {
        (*self.role.borrow()).is_some_and(ClientRole::is_presenter)
    }

    /// Navigates to the next slide, skipping any reveals left on this one.
    ///
    /// Returns once the server has applied the move, so reading `state`
    /// straight afterwards reports the slide this call landed on.
    ///
    /// # Errors
    /// Raises `PermissionError` if this client is watching rather than
    /// presenting, `RuntimeError` if the server rejects the command — an
    /// out-of-range `goto`, a deck with no slides — and `ConnectionError` if
    /// the server cannot be reached.
    pub fn next(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::NextSlide)
    }

    /// Navigates to the previous slide.
    ///
    /// # Errors
    /// As [`Toboggan::next`].
    pub fn previous(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::PreviousSlide)
    }

    /// Navigate to the first slide.
    ///
    /// # Errors
    /// As [`Toboggan::next`].
    pub fn first(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::First)
    }

    /// Navigate to the last slide.
    ///
    /// # Errors
    /// As [`Toboggan::next`].
    pub fn last(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::Last)
    }

    /// Navigate to a specific slide (1-indexed).
    ///
    /// # Errors
    /// As [`Toboggan::next`]. A slide number the deck does not have is a
    /// `RuntimeError` rather than a silent no-op.
    pub fn goto(&self, py: Python<'_>, slide: usize) -> PyResult<()> {
        self.drive(py, goto_command(slide))
    }

    /// Move to the next step within the current slide.
    ///
    /// # Errors
    /// As [`Toboggan::next`].
    pub fn next_step(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::NextStep)
    }

    /// Move to the previous step within the current slide.
    ///
    /// # Errors
    /// As [`Toboggan::next`].
    pub fn previous_step(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::PreviousStep)
    }

    /// Trigger a visual blink effect.
    ///
    /// # Errors
    /// As [`Toboggan::next`]. A blink moves nothing, so it leaves `state`
    /// alone.
    pub fn blink(&self, py: Python<'_>) -> PyResult<()> {
        self.drive(py, Command::Blink)
    }

    /// Get list of connected clients.
    ///
    /// Presenter-only on the server — it reports names, roles and IP
    /// addresses — so an audience connection gets an error here rather than a
    /// list.
    ///
    /// # Errors
    /// Raises `PermissionError` if this client is watching rather than
    /// presenting, and `ConnectionError` if the server cannot be reached.
    pub fn clients(&self, py: Python<'_>) -> PyResult<Vec<ClientInfo>> {
        let api = self.api.clone();
        let handle = self.rt.handle().clone();

        let clients = py
            .detach(move || handle.block_on(async move { api.clients().await }))
            .map_err(|err| refused_or_unreachable(&err))?;

        Ok(clients.into_iter().map(ClientInfo).collect())
    }

    pub fn __repr__(&self) -> String {
        format!("Toboggan({:?})", self.config)
    }

    pub fn __str__(&self) -> String {
        format!("Toboggan({})", self.config.api_url())
    }
}

/// A failed request, as the Python exception that fits it.
///
/// A `403` is not a connection problem: the server understood perfectly and
/// said no, because this client is watching rather than presenting. Reporting
/// that as `ConnectionError` would send a reader hunting for a network fault
/// that is not there.
fn refused_or_unreachable(err: &TobogganApiError) -> PyErr {
    let TobogganApiError::ReqwestError(reqwest_error) = err;
    if reqwest_error.status().is_some_and(|status| status == 403) {
        return PyPermissionError::new_err(
            "This client is watching, not presenting. Connect from the machine \
             running the server, or pass a presenter token.",
        );
    }
    PyConnectionError::new_err(err.to_string())
}

/// The token to offer at registration, from the argument or the environment.
///
/// [`Secret::new`] is what decides whether a string is a usable token, on both
/// sides of the wire — so a token that is blank, or whitespace, is no token
/// rather than one the server can only refuse.
fn resolve_token(presenter_token: Option<&str>) -> Option<Secret> {
    match presenter_token {
        Some(token) => Secret::new(token),
        None => std::env::var(PRESENTER_TOKEN_ENV)
            .ok()
            .as_deref()
            .and_then(Secret::new),
    }
}

async fn handle_state(
    state: Arc<RwLock<TState>>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    role: watch::Sender<Option<ClientRole>>,
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
                // Only ever *other* clients' errors now: this client's commands
                // travel over `/api/command`, which answers the caller, so its
                // own failures are raised in Python rather than printed here.
                eprintln!("🚨 Oops: {error}");
            }
            CommunicationMessage::Registered {
                client_id,
                role: granted,
            } => {
                // Re-sent on every reconnect, so this tracks the role rather
                // than recording the first one: a server restarted with a
                // different `--presenter-token` demotes this client, and a
                // stale `is_presenter` would say the commands still work.
                let _ = role.send(Some(granted));
                println!(
                    "🆔 Registered as {} with id: {client_id:?}",
                    role_name(granted)
                );
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
