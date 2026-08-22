use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use pyo3::exceptions::{PyConnectionError, PyPermissionError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use toboggan_client::{
    CommunicationMessage, StatusCode, TobogganApi, TobogganApiError, TobogganConfig,
    WebSocketClient,
};
use toboggan_core::{
    ClientConfig, ClientRole, Command, Notification, Secret, SlidesResponse, State as TState,
    TalkResponse, goto_command,
};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::{RwLock, watch};
use tokio::try_join;
use tracing::{debug, error, info, warn};

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

/// How long the constructor waits for the socket to come up.
///
/// [`REGISTRATION_TIMEOUT`] is bounded so `Toboggan(...)` cannot hang in a
/// REPL, but it is only reached once the socket is up and the deck is fetched —
/// and the socket was not bounded, so the wait it was written to prevent
/// happened one step earlier. A server that completes the TCP handshake and
/// then never answers the upgrade leaves `connect_async` waiting forever, and
/// because the call is detached from the GIL, Ctrl-C only sets a flag.
///
/// Just the socket: the REST fetches carry their own timeouts from
/// `TobogganApi`, and `/api/slides` on a large deck deserves a longer budget
/// than a handshake ever does.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Toboggan presentation client.
///
/// Navigation is synchronous: when [`Toboggan::next`] and its siblings return,
/// the server has applied the command and the [`Toboggan::state`] getter
/// reports where the deck now is. The socket is still there, doing the job only
/// it can do — reporting moves *other* clients made, and deck reloads.
#[pyclass]
pub struct Toboggan {
    config: TobogganConfig,
    /// Declared *before* `rt`, because fields drop in declaration order and the
    /// socket has to go first. `Runtime::drop` blocks until its threads are
    /// done, and a `Toboggan` dropped by the garbage collector runs that with
    /// the GIL held — freezing the interpreter for as long as the shutdown
    /// takes, at the worst possible moment, with the reconnect loop still
    /// running. [`Toboggan::close`] is the way to avoid finding out.
    _ws: WebSocketClient,
    /// `Option` so `close` can take it and shut it down deliberately.
    rt: Option<Runtime>,
    api: TobogganApi,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    state: Arc<RwLock<TState>>,
    /// Set when a deck reload arrived and the new deck could not be fetched.
    ///
    /// The caches then hold the last snapshot that agreed with itself, which is
    /// no longer what the server is serving. Reported rather than papered over:
    /// silently handing back a slide from a deck that has been replaced is the
    /// kind of wrong answer nobody traces back here.
    deck_stale: Arc<AtomicBool>,
    /// How many commands are waiting on `/api/command` right now.
    ///
    /// Read by the listener to tell this client's own echo from a third party's
    /// move. See the `StateChange` arm in [`handle_state`].
    in_flight: Arc<AtomicUsize>,
    role: watch::Receiver<Option<ClientRole>>,
}

impl Toboggan {
    /// The runtime, or the reason there no longer is one.
    ///
    /// Every call needs it, and after [`Toboggan::close`] none of them can
    /// work — so say that plainly rather than panicking on an `Option` nobody
    /// expected to be empty.
    fn runtime(&self) -> PyResult<&Runtime> {
        self.rt.as_ref().ok_or_else(|| {
            PyRuntimeError::new_err("this client is closed; create a new Toboggan to reconnect")
        })
    }

    /// Refuses to answer from a cache known to disagree with the server.
    ///
    /// Set when a deck reload arrived and the refetch that should have followed
    /// it failed. What is cached is then the *previous* deck — coherent with
    /// itself, and no longer what anyone is presenting. Handing it over would
    /// answer questions about a deck that has been replaced, which is a wrong
    /// answer wearing the shape of a right one.
    ///
    /// Clears itself: the next successful reload puts the caches back in step.
    fn fresh(&self) -> PyResult<()> {
        if self.deck_stale.load(Ordering::Acquire) {
            return Err(PyRuntimeError::new_err(
                "the deck was reloaded but could not be refetched, so what is \
                 cached here is the deck as it was before. Retry once the \
                 server is reachable again.",
            ));
        }
        Ok(())
    }

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
        let in_flight = Arc::clone(&self.in_flight);
        let handle = self.runtime()?.handle().clone();

        // Detached: this is a network round trip, and a `block_on` that keeps
        // the GIL freezes every other Python thread for its whole duration.
        let notification = py
            .detach(move || {
                handle.block_on(async move {
                    // Raised before the request and lowered only after the
                    // cache is written, so the listener can recognise this
                    // command's own echo for as long as it might arrive.
                    in_flight.fetch_add(1, Ordering::AcqRel);
                    let sent = api.command(command).await;

                    if let Ok(
                        Notification::State { state: applied }
                        | Notification::TalkChange { state: applied },
                    ) = &sent
                    {
                        *state.write().await = applied.clone();
                    }

                    in_flight.fetch_sub(1, Ordering::AcqRel);
                    sent
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
    /// Raises `ConnectionError` if the server cannot be reached, `RuntimeError`
    /// if it answers with a refusal or something this client cannot read, and
    /// `PermissionError` if it answers `403` — the same taxonomy every other
    /// call uses, so that a token the server rejects does not read as a network
    /// fault here and a permission problem there.
    ///
    /// A registration that goes unanswered is *not* an error: the deck is
    /// there, and `role` reports None until it arrives. Nor is a socket that
    /// does not come up in time — the reconnect loop keeps trying.
    ///
    /// Raises `OSError` if the async runtime cannot be started, and
    /// `OverflowError` for a port outside `0..=65535`.
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
        let deck_stale = Arc::<AtomicBool>::default();
        let in_flight = Arc::<AtomicUsize>::default();
        let (role_tx, mut role_rx) = watch::channel(None);

        // Detached for the same reason as `drive`: connecting and fetching the
        // deck is network work, and the registration wait below can take
        // seconds. Holding the GIL across either freezes the interpreter — a
        // constructor is no better a place to do that than a method.
        let (connected, fetched) = py
            .detach(|| {
                rt.block_on(async {
                    let _read_messages = tokio::spawn(handle_state(
                        Caches {
                            state: Arc::clone(&state),
                            talk: Arc::clone(&talk),
                            slides: Arc::clone(&slides),
                            deck_stale: Arc::clone(&deck_stale),
                            in_flight: Arc::clone(&in_flight),
                        },
                        role_tx,
                        api.clone(),
                        rx_msg,
                    ));
                    // `connect` cannot report failure — it reconnects forever
                    // by design — so a bound here is the only thing standing
                    // between a silent server and a constructor that never
                    // returns. Expiry is not fatal on its own: the reconnect
                    // loop keeps trying, and the deck fetch below decides
                    // whether the server is reachable at all.
                    let connected = tokio::time::timeout(CONNECT_TIMEOUT, ws.connect())
                        .await
                        .is_ok();
                    (connected, try_join!(api.talk(), api.slides()))
                })
            });

        if !connected {
            warn!(
                host,
                port,
                seconds = CONNECT_TIMEOUT.as_secs(),
                "the socket did not come up in time; moves made by other \
                 clients will not be seen until it does"
            );
        }

        let (initial_talk, initial_slides) = fetched.map_err(|err| refused_or_unreachable(&err))?;

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
                    warn!(
                        seconds = REGISTRATION_TIMEOUT.as_secs(),
                        "the server has not answered registration; role unknown for now"
                    );
                }
            });
        });

        Ok(Self {
            rt: Some(rt),
            config,
            _ws: ws,
            api,
            talk,
            slides,
            state,
            deck_stale,
            in_flight,
            role: role_rx,
        })
    }

    /// Gets the presentation metadata.
    ///
    /// # Errors
    /// Raises `RuntimeError` if the deck was reloaded and could not be
    /// refetched; see [`Toboggan::fresh`].
    #[getter]
    pub fn talk(&self) -> PyResult<Talk> {
        self.fresh()?;
        let talk = Arc::clone(&self.talk);
        let talk = self.runtime()?.block_on(async {
            let guard = talk.read().await;
            TalkResponse::clone(&guard)
        });
        Ok(Talk(talk))
    }

    /// Gets all slides in the presentation.
    ///
    /// # Errors
    /// Raises `RuntimeError` if the deck was reloaded and could not be
    /// refetched; see [`Toboggan::fresh`].
    #[getter]
    pub fn slides(&self) -> PyResult<Slides> {
        self.fresh()?;
        let slides = Arc::clone(&self.slides);
        let slides = self.runtime()?.block_on(async {
            let guard = slides.read().await;
            SlidesResponse::clone(&guard)
        });
        Ok(Slides(slides))
    }

    /// Where the deck is now.
    ///
    /// Trustworthy immediately after a navigation call: those return only once
    /// the server has applied the command and this cache holds its answer.
    ///
    /// # Errors
    /// Raises `RuntimeError` if the deck was reloaded and could not be
    /// refetched; see [`Toboggan::fresh`].
    #[getter]
    pub fn state(&self) -> PyResult<State> {
        self.fresh()?;
        let state = Arc::clone(&self.state);
        let slides = Arc::clone(&self.slides);

        // Both in one `block_on`, so `is_last_slide` is answered against the
        // deck this state belongs to. Reading them from Python as two separate
        // calls — which is what taking the count as an argument forced — left a
        // deck reload free to land in between.
        let (state, total_slides) = self.runtime()?.block_on(async {
            let state = TState::clone(&*state.read().await);
            let total_slides = slides.read().await.slides.len();
            (state, total_slides)
        });

        Ok(State(state, total_slides))
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
    /// `RuntimeError` rather than a silent no-op, and `0` is a `ValueError`.
    pub fn goto(&self, py: Python<'_>, slide: usize) -> PyResult<()> {
        // `goto_command` does `saturating_sub(1)`, so `0` lands on slide 1
        // instead of failing — which made the one number a caller carrying a
        // 0-based index would actually pass the one number that moved the deck
        // silently to the wrong place. Every *other* out-of-range value raises.
        if slide == 0 {
            return Err(PyValueError::new_err(
                "slide numbers count from 1; 0 is not a slide",
            ));
        }
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
        let handle = self.runtime()?.handle().clone();

        let clients = py
            .detach(move || handle.block_on(async move { api.clients().await }))
            .map_err(|err| refused_or_unreachable(&err))?;

        Ok(clients.into_iter().map(ClientInfo).collect())
    }

    /// Disconnects and shuts the client's runtime down.
    ///
    /// Idempotent, and every other call raises `RuntimeError` afterwards.
    ///
    /// Worth calling rather than leaving to the garbage collector: dropping a
    /// `Toboggan` runs `Runtime::drop`, which blocks until its worker threads
    /// finish — and the collector runs that with the GIL held, so an
    /// interpreter can sit frozen in a shutdown nobody asked for. Doing it here
    /// releases the GIL first, which is the whole difference.
    pub fn close(&mut self, py: Python<'_>) {
        if let Some(rt) = self.rt.take() {
            py.detach(|| drop(rt));
        }
    }

    fn __enter__(this: PyRef<'_, Self>) -> PyRef<'_, Self> {
        this
    }

    /// Closes on the way out, however the block ended.
    ///
    /// Returns `false`, so an exception raised inside the `with` propagates —
    /// closing the client is not a reason to swallow it.
    #[pyo3(signature = (*_args))]
    fn __exit__(&mut self, py: Python<'_>, _args: &Bound<'_, PyAny>) -> bool {
        self.close(py);
        false
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
/// Only a transport failure is a `ConnectionError`. A `403` is not: the server
/// understood perfectly and said no, and reporting that as unreachable sends a
/// reader hunting for a network fault that is not there. Nor is a body this
/// client cannot parse, which means the two ends disagree about a shape — the
/// failure that hid in `clients()` for as long as it did precisely because it
/// arrived in Python wearing a `ConnectionError`.
///
/// Exhaustive on purpose: a new [`TobogganApiError`] variant should fail to
/// compile here rather than fall into a catch-all and be mislabelled.
fn refused_or_unreachable(err: &TobogganApiError) -> PyErr {
    match err {
        TobogganApiError::Transport(_) => PyConnectionError::new_err(err.to_string()),

        TobogganApiError::Status { code, body } if *code == StatusCode::FORBIDDEN => {
            // The server has more than one reason to refuse — the presenter
            // gate and the origin guard both answer 403 — so prefer whatever it
            // said over guessing which one it was.
            let explanation = if body.is_empty() {
                "This client is watching, not presenting. Connect from the \
                 machine running the server, or pass a presenter token."
                    .to_owned()
            } else {
                body.clone()
            };
            PyPermissionError::new_err(explanation)
        }

        TobogganApiError::Status { .. } | TobogganApiError::Decode(_) => {
            PyRuntimeError::new_err(err.to_string())
        }
    }
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

/// Everything the background listener writes to, in one place.
///
/// A struct rather than seven arguments because they are one thing: the caches
/// this client serves its getters from, and the two flags that say whether they
/// can be trusted.
struct Caches {
    state: Arc<RwLock<TState>>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    deck_stale: Arc<AtomicBool>,
    in_flight: Arc<AtomicUsize>,
}

async fn handle_state(
    caches: Caches,
    role: watch::Sender<Option<ClientRole>>,
    api: TobogganApi,
    mut rx: UnboundedReceiver<CommunicationMessage>,
) {
    debug!("listening for pushed messages");

    while let Some(msg) = rx.recv().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange { status } => {
                debug!(%status, "connection status changed");
            }

            CommunicationMessage::StateChange { state: new_state } => {
                // The server broadcasts before it answers the caller, and does
                // not exclude the sender — so this frame may be the echo of a
                // command `drive` is still waiting on. Applying it then would
                // race `drive`'s own write and could put the cache *back* to
                // where the deck was before the call, which is precisely the
                // staleness synchronous navigation exists to rule out.
                //
                // This narrows the window rather than closing it: the server
                // sends no sequence number, so an echo delayed past a whole
                // REST round trip can still arrive after the command that
                // followed it. Closing it properly needs ordering the server
                // does not currently provide.
                if caches.in_flight.load(Ordering::Acquire) > 0 {
                    debug!("ignoring a frame that arrived while a command was in flight");
                    continue;
                }
                *caches.state.write().await = new_state;
            }

            CommunicationMessage::TalkChange { state: new_state } => {
                info!("the deck changed; refetching it");

                match try_join!(api.talk(), api.slides()) {
                    Ok((new_talk, new_slides)) => {
                        // Three separate locks, so a getter can still catch the
                        // new talk against the old slides. Deliberately not
                        // called atomic — it is not.
                        *caches.talk.write().await = new_talk;
                        *caches.slides.write().await = new_slides;
                        *caches.state.write().await = new_state;
                        caches.deck_stale.store(false, Ordering::Release);
                        info!("the deck is up to date");
                    }
                    Err(err) => {
                        // The new state indexes into the *new* deck, and the
                        // new deck is what we just failed to fetch. Committing
                        // it against the old `talk`/`slides` would leave the
                        // caches quietly disagreeing with each other —
                        // `state.slide` pointing into a deck that no longer
                        // exists, which reads much later as "toboggan sometimes
                        // reports the wrong slide title" with no thread to pull.
                        //
                        // So: keep the last coherent snapshot, and mark it
                        // untrustworthy so the getters say so out loud.
                        error!(%err, "the deck changed but could not be refetched");
                        caches.deck_stale.store(true, Ordering::Release);
                    }
                }
            }

            CommunicationMessage::Error { error } => {
                // Another client's failure: this client's commands travel over
                // `/api/command`, which answers its caller, so its own errors
                // are raised in Python rather than arriving here.
                warn!(%error, "the server reported an error for another client");
            }

            CommunicationMessage::Registered {
                client_id,
                role: granted,
            } => {
                // Re-sent on every reconnect, so this tracks the role rather
                // than recording the first one: a server restarted with a
                // different `--presenter-token` demotes this client, and a
                // stale `is_presenter` would say the commands still work.
                //
                // Fails only once every receiver is gone — that is, once the
                // `Toboggan` has been dropped and this task is on its way out.
                let _no_receivers_left = role.send(Some(granted));
                debug!(role = role_name(granted), ?client_id, "registered");
            }

            CommunicationMessage::ClientConnected { client_id, name } => {
                debug!(%name, ?client_id, "a client joined");
            }

            CommunicationMessage::ClientDisconnected { client_id, name } => {
                debug!(%name, ?client_id, "a client left");
            }
        }
    }

    debug!("the message stream ended");
}
