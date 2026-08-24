use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use pyo3::exceptions::{PyConnectionError, PyPermissionError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use toboggan_client::{
    CommunicationMessage, ConnectionStatus, StatusCode, TobogganApi, TobogganApiError,
    TobogganConfig, WebSocketClient,
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
    /// socket has to go first.
    _ws: WebSocketClient,
    /// `Option` so `close` and `drop` can each take it exactly once.
    rt: Option<Runtime>,
    api: TobogganApi,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    state: Arc<watch::Sender<Cached>>,
    /// Set when a deck reload arrived and the new deck could not be fetched.
    ///
    /// The caches then hold the last snapshot that agreed with itself, which is
    /// no longer what the server is serving. Reported rather than papered over:
    /// silently handing back a slide from a deck that has been replaced is the
    /// kind of wrong answer nobody traces back here.
    deck_stale: Arc<AtomicBool>,
    role: watch::Receiver<Option<ClientRole>>,
}

/// The last state this client believes, and the server's number for it.
///
/// This client is the only one in the workspace that learns the state over two
/// channels — the socket, and the body of `POST /api/command` — which arrive on
/// separate connections in no fixed order. The number is what orders them.
#[derive(Default)]
struct Cached {
    /// `None` until the server has said where the deck is.
    ///
    /// Not a `State::Init` standing in for "no idea yet": those are different
    /// answers, and only one of them is ever true. A deck sitting on slide five
    /// behind a socket that has not come up would otherwise be reported as not
    /// started — a wrong answer wearing the shape of a right one, and
    /// indistinguishable from the real thing.
    state: Option<TState>,
    seq: u64,
}

impl Cached {
    /// Forgets which change this state was, keeping the state itself.
    ///
    /// The number counts changes on one server process, so it means nothing
    /// across a restart — and a client still holding the old server's number
    /// would refuse everything a new one says, silently and for good. Called
    /// when the socket drops, because that is the moment the number stops
    /// referring to anything.
    const fn forget_which_change(&mut self) {
        self.seq = Notification::UNNUMBERED;
    }
}

/// Writes a state unless something at least as new is already cached.
///
/// The comparison happens under the write lock so it cannot be split from the
/// write it guards; two callers racing here is the entire situation this exists
/// for.
///
/// `>=` rather than `>` for two reasons. A re-broadcast of the same change is
/// then idempotent, and a server that numbers nothing — every frame
/// [`Notification::UNNUMBERED`] — degrades to applying everything, which is what
/// every other client in the workspace does and what this one did before.
///
/// Returns whether the write happened, which the listener logs.
fn accept(cache: &watch::Sender<Cached>, state: TState, seq: u64) -> bool {
    cache.send_if_modified(|cached| {
        if seq < cached.seq {
            return false;
        }
        *cached = Cached {
            state: Some(state),
            seq,
        };
        true
    })
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
    /// Clears itself: the next successful refetch puts the caches back in
    /// step, and the socket reconnecting is enough to trigger one.
    fn fresh(&self) -> PyResult<()> {
        if self.deck_stale.load(Ordering::Acquire) {
            return Err(PyRuntimeError::new_err(
                "the deck was reloaded but could not be refetched, so what is \
                 cached here is the deck as it was before. The client refetches \
                 whenever the socket reconnects; retry once the server is \
                 reachable again.",
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
    /// A third party moving the deck at the same moment does not disturb that.
    /// Its frame and this answer both carry the server's sequence number, and
    /// [`accept`] takes whichever is newer — so if the deck has already moved
    /// past this command by the time the answer arrives, the cache keeps the
    /// later position rather than being dragged back to this one.
    fn drive(&self, py: Python<'_>, command: Command) -> PyResult<()> {
        let api = self.api.clone();
        let cache = Arc::clone(&self.state);
        let handle = self.runtime()?.handle().clone();

        // Detached: this is a network round trip, and a `block_on` that keeps
        // the GIL freezes every other Python thread for its whole duration.
        let notification = py
            .detach(move || {
                handle.block_on(async move {
                    let sent = api.command(command).await;

                    if let Ok(
                        Notification::State { state, seq }
                        | Notification::TalkChange { state, seq },
                    ) = &sent
                    {
                        accept(&cache, state.clone(), *seq);
                    }

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

        let state = Arc::new(watch::Sender::new(Cached::default()));
        let mut state_rx = state.subscribe();
        let talk = Arc::<RwLock<TalkResponse>>::default();
        let slides = Arc::<RwLock<SlidesResponse>>::default();
        let deck_stale = Arc::<AtomicBool>::default();
        let (role_tx, mut role_rx) = watch::channel(None);

        // Detached for the same reason as `drive`: connecting and fetching the
        // deck is network work, and the registration wait below can take
        // seconds. Holding the GIL across either freezes the interpreter — a
        // constructor is no better a place to do that than a method.
        let (connected, fetched) = py.detach(|| {
            rt.block_on(async {
                let _read_messages = tokio::spawn(handle_state(
                    Caches {
                        state: Arc::clone(&state),
                        talk: Arc::clone(&talk),
                        slides: Arc::clone(&slides),
                        deck_stale: Arc::clone(&deck_stale),
                    },
                    role_tx,
                    api.clone(),
                    rx_msg,
                ));
                // Bounded by `connect_within` rather than by a `timeout`
                // around `connect`: cancelling that future drops it before it
                // has spawned the retry loop, so a socket that missed its
                // budget would never come up at all. Expiry is not fatal on
                // its own — the retry loop is running, and the deck fetch
                // below decides whether the server is reachable at all.
                let connected = ws.connect_within(Some(CONNECT_TIMEOUT)).await;
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

        let (initial_talk, initial_slides) = match fetched {
            Ok(fetched) => fetched,
            Err(err) => {
                // Detached: dropping the runtime here is the same blocking
                // shutdown `close` exists to keep off the GIL, and the error
                // path is no better a place to freeze the interpreter than the
                // happy one.
                py.detach(|| drop(rt));
                return Err(refused_or_unreachable(&err));
            }
        };

        py.detach(|| {
            rt.block_on(async {
                *talk.write().await = initial_talk;
                *slides.write().await = initial_slides;

                // Both, on one budget: the server sends `Registered` before it
                // sends the first state, so waiting only for the role can wake
                // while the client still has no idea where the deck is — and
                // the `state` getter would then have nothing to answer with.
                //
                // A timeout is not a failure. The deck was fetched, so the
                // client is usable; it just does not know yet what it is
                // allowed to do, or where the deck is. Both say so honestly.
                let settled = async {
                    role_rx.wait_for(Option::is_some).await.ok();
                    state_rx
                        .wait_for(|cached| cached.state.is_some())
                        .await
                        .ok();
                };
                if tokio::time::timeout(REGISTRATION_TIMEOUT, settled)
                    .await
                    .is_err()
                {
                    warn!(
                        seconds = REGISTRATION_TIMEOUT.as_secs(),
                        "the server has not finished answering; role or position unknown for now"
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
    /// refetched (see [`Toboggan::fresh`]), or if the server has not yet said
    /// where the deck is — which is not the same answer as "it has not
    /// started", and must not be reported as one.
    #[getter]
    pub fn state(&self) -> PyResult<State> {
        self.fresh()?;
        let state = Arc::clone(&self.state);
        let slides = Arc::clone(&self.slides);

        // The deck guard is taken first and held across the borrow, so the two
        // halves cannot come from either side of a reload: a `TalkChange`
        // writes the slides before it accepts the state it belongs to, and
        // blocks here until this pair has been read.
        let (state, total_slides) = self.runtime()?.block_on(async {
            let deck = slides.read().await;
            let cached = state.borrow();
            (cached.state.clone(), deck.slides.len())
        });

        let state = state.ok_or_else(|| {
            PyRuntimeError::new_err(
                "the server has not said where the deck is yet. The socket \
                 carries that, and it has not come up — the client keeps \
                 trying, so retry shortly.",
            )
        })?;

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
    /// Idempotent, and every call that needs the server raises `RuntimeError`
    /// afterwards.
    ///
    /// Worth calling rather than leaving to the garbage collector, which cannot
    /// wait for the worker threads at all: this releases the GIL and waits, so
    /// the socket is closed and the threads are gone by the time it returns.
    /// See [`Toboggan::drop`] for what the collector does instead.
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

impl Drop for Toboggan {
    /// Shuts the runtime down without waiting for it.
    ///
    /// A `Toboggan` the garbage collector reclaims is dropped with the GIL
    /// held, and `Runtime::drop` blocks until its worker threads finish — so
    /// the plain drop freezes the interpreter for the length of a shutdown
    /// nobody asked for, at a moment nobody chose. Worse now that `pyo3_log`
    /// lets those threads reach for the GIL on their way out, which is a thread
    /// waiting on a lock held by a thread waiting on it.
    ///
    /// `shutdown_background` returns immediately and lets the threads wind down
    /// on their own. The socket closes either way; what is given up is knowing
    /// when. [`Toboggan::close`] is how a caller asks to know.
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
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
/// A struct rather than four arguments because they are one thing: the caches
/// this client serves its getters from, and the flag that says whether they can
/// be trusted.
struct Caches {
    state: Arc<watch::Sender<Cached>>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    deck_stale: Arc<AtomicBool>,
}

/// Pulls the deck into the caches, reporting whether they now agree with the
/// server.
///
/// A failure leaves the previous snapshot in place. It is coherent with itself,
/// which a half-written pair would not be — and the caller marks it
/// untrustworthy rather than handing it back as though nothing had happened.
async fn refetch_deck(caches: &Caches, api: &TobogganApi) -> bool {
    match try_join!(api.talk(), api.slides()) {
        Ok((new_talk, new_slides)) => {
            // Two separate locks, so a getter can still catch the new talk
            // against the old slides. Deliberately not called atomic — it is
            // not.
            *caches.talk.write().await = new_talk;
            *caches.slides.write().await = new_slides;
            true
        }
        Err(err) => {
            error!(%err, "the deck could not be refetched");
            false
        }
    }
}

async fn handle_state(
    caches: Caches,
    role: watch::Sender<Option<ClientRole>>,
    api: TobogganApi,
    mut rx: UnboundedReceiver<CommunicationMessage>,
) {
    debug!("listening for pushed messages");

    // The connection the constructor waited for does not need refetching — the
    // constructor fetched the deck itself. Every *later* one does; see below.
    let mut seen_connection = false;

    while let Some(msg) = rx.recv().await {
        match msg {
            CommunicationMessage::ConnectionStatusChange {
                status: ConnectionStatus::Connected,
            } => {
                debug!("connected");

                if std::mem::replace(&mut seen_connection, true) {
                    // A reconnect, and the deck may have been rebuilt while the
                    // socket was down. The server replays the current *state*
                    // on a new socket but not the `TalkChange` that was missed,
                    // so nothing else would ever tell this client — it would
                    // answer from the old deck, coherent and wrong, with no
                    // error and nothing to pull on.
                    //
                    // This is also the only thing that clears `deck_stale`
                    // after a refetch failed, which is what `fresh` tells the
                    // caller to wait for.
                    info!("reconnected; refetching the deck");
                    let agrees = refetch_deck(&caches, &api).await;
                    caches.deck_stale.store(!agrees, Ordering::Release);
                }
            }

            CommunicationMessage::ConnectionStatusChange { status } => {
                debug!(%status, "connection status changed");

                // A restarted server counts from zero again, and a client still
                // holding the old server's number would reject every frame the
                // new one ever sends — silently, and for good. The number means
                // something only for as long as the connection that issued it,
                // so when that connection goes, so does the baseline.
                //
                // Nothing is lost by clearing it: the server replays the
                // current state as the first frame on the new socket, which
                // puts a real number back.
                caches.state.send_if_modified(|cached| {
                    cached.forget_which_change();
                    // The state itself is unchanged, so nobody waiting on one
                    // needs waking.
                    false
                });
            }

            CommunicationMessage::StateChange {
                state: new_state,
                seq,
            } => {
                // The server broadcasts before it answers the caller and
                // excludes nobody, so this frame may be the echo of a command
                // `drive` is still waiting on — arriving either side of
                // `drive`'s own write, on a different connection.
                //
                // Which is why neither of them decides: the number does.
                if !accept(&caches.state, new_state, seq) {
                    debug!(seq, "ignoring a frame older than what is cached");
                }
            }

            CommunicationMessage::TalkChange {
                state: new_state,
                seq,
            } => {
                info!("the deck changed; refetching it");

                if refetch_deck(&caches, &api).await {
                    accept(&caches.state, new_state, seq);
                    caches.deck_stale.store(false, Ordering::Release);
                    info!("the deck is up to date");
                } else {
                    // The new state indexes into the *new* deck, and the new
                    // deck is what we just failed to fetch. Committing it
                    // against the old `talk`/`slides` would leave the caches
                    // quietly disagreeing with each other — `state.slide`
                    // pointing into a deck that no longer exists, which reads
                    // much later as "toboggan sometimes reports the wrong slide
                    // title" with no thread to pull.
                    //
                    // So: keep the last coherent snapshot, and mark it
                    // untrustworthy so the getters say so out loud.
                    caches.deck_stale.store(true, Ordering::Release);
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::SlideId;

    use super::*;

    fn running(slide: usize) -> TState {
        TState::Running {
            current: SlideId::new(slide - 1),
            current_step: 0,
        }
    }

    fn cached_slide(cache: &watch::Sender<Cached>) -> usize {
        let cached = cache.borrow();
        let state = cached.state.as_ref().expect("a state to have arrived");
        state.current().expect("a running state").index() + 1
    }

    /// The echo race, which is why this rule exists.
    ///
    /// The server broadcasts before it answers, and excludes nobody — so the
    /// frame for an *earlier* move can still be climbing this client's socket
    /// while its own later command is being answered over REST. Whichever
    /// lands second used to win; now the newer one does.
    #[test]
    fn a_late_frame_does_not_undo_a_newer_answer() {
        let cache = watch::Sender::new(Cached::default());

        // Somebody else moved to slide 2; then this client's goto(7) was
        // applied and answered.
        assert!(accept(&cache, running(2), 5));
        assert!(accept(&cache, running(7), 6));

        // The broadcast of that first move finally arrives.
        assert!(
            !accept(&cache, running(2), 5),
            "an older frame must be refused, not merely lose a race"
        );
        assert_eq!(
            cached_slide(&cache),
            7,
            "an echo overwrote the answer the command produced"
        );
    }

    /// The other half, and the one an in-flight guard used to get wrong.
    ///
    /// A move somebody else made *after* this client's command must reach the
    /// cache. Dropping it leaves this client reporting a slide the deck has
    /// left, with nothing to correct it until the next change — which may be
    /// minutes away, or never.
    #[test]
    fn a_newer_frame_wins_over_an_older_answer() {
        let cache = watch::Sender::new(Cached::default());

        // This client's goto(7) is answered over REST...
        assert!(accept(&cache, running(7), 6));
        // ...and a move made after it arrives on the socket.
        assert!(accept(&cache, running(2), 7));

        assert_eq!(cached_slide(&cache), 2, "a third party's move was lost");
    }

    /// A restarted server counts from zero, and must not be locked out.
    ///
    /// Without the reset the baseline outlives the connection that issued it,
    /// and every frame the new server sends is numbered below it — so the
    /// client refuses all of them, for good, while looking perfectly healthy.
    #[test]
    fn a_dropped_socket_clears_the_baseline() {
        let cache = watch::Sender::new(Cached::default());

        assert!(accept(&cache, running(9), 12));
        // Without this the assertion below fails, which is the whole point.
        cache.send_if_modified(|cached| {
            cached.forget_which_change();
            false
        });

        assert!(
            accept(&cache, running(3), 1),
            "a restarted server's first change was refused as stale"
        );
        assert_eq!(cached_slide(&cache), 3);
    }

    /// A server that numbers nothing behaves as it did before there were
    /// numbers: every frame applied, in arrival order.
    #[test]
    fn an_unnumbered_server_still_moves_the_deck() {
        let cache = watch::Sender::new(Cached::default());

        for slide in [3, 1, 9, 4] {
            assert!(accept(&cache, running(slide), Notification::UNNUMBERED));
            assert_eq!(cached_slide(&cache), slide);
        }
    }
}
