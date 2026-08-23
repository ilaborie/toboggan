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
    state: Arc<RwLock<Cached>>,
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
    state: TState,
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
async fn accept(cache: &RwLock<Cached>, state: TState, seq: u64) -> bool {
    let mut cached = cache.write().await;
    if seq < cached.seq {
        return false;
    }
    *cached = Cached { state, seq };
    true
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
                        accept(&cache, state.clone(), *seq).await;
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

        let state = Arc::<RwLock<Cached>>::default();
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
            let state = TState::clone(&state.read().await.state);
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
    state: Arc<RwLock<Cached>>,
    talk: Arc<RwLock<TalkResponse>>,
    slides: Arc<RwLock<SlidesResponse>>,
    deck_stale: Arc<AtomicBool>,
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

                // A restarted server counts from zero again, and a client still
                // holding the old server's number would reject every frame the
                // new one ever sends — silently, and for good. The number means
                // something only for as long as the connection that issued it,
                // so when that connection goes, so does the baseline.
                //
                // Nothing is lost by clearing it: the server replays the
                // current state as the first frame on the new socket, which
                // puts a real number back.
                if !matches!(status, ConnectionStatus::Connected) {
                    caches.state.write().await.forget_which_change();
                }
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
                if !accept(&caches.state, new_state, seq).await {
                    debug!(seq, "ignoring a frame older than what is cached");
                }
            }

            CommunicationMessage::TalkChange {
                state: new_state,
                seq,
            } => {
                info!("the deck changed; refetching it");

                match try_join!(api.talk(), api.slides()) {
                    Ok((new_talk, new_slides)) => {
                        // Three separate locks, so a getter can still catch the
                        // new talk against the old slides. Deliberately not
                        // called atomic — it is not.
                        *caches.talk.write().await = new_talk;
                        *caches.slides.write().await = new_slides;
                        accept(&caches.state, new_state, seq).await;
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

    async fn cached_slide(cache: &RwLock<Cached>) -> usize {
        let cached = cache.read().await;
        cached.state.current().expect("a running state").index() + 1
    }

    /// The echo race, which is why this rule exists.
    ///
    /// The server broadcasts before it answers, and excludes nobody — so the
    /// frame for an *earlier* move can still be climbing this client's socket
    /// while its own later command is being answered over REST. Whichever
    /// lands second used to win; now the newer one does.
    #[tokio::test]
    async fn a_late_frame_does_not_undo_a_newer_answer() {
        let cache = RwLock::new(Cached::default());

        // Somebody else moved to slide 2; then this client's goto(7) was
        // applied and answered.
        assert!(accept(&cache, running(2), 5).await);
        assert!(accept(&cache, running(7), 6).await);

        // The broadcast of that first move finally arrives.
        assert!(
            !accept(&cache, running(2), 5).await,
            "an older frame must be refused, not merely lose a race"
        );
        assert_eq!(
            cached_slide(&cache).await,
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
    #[tokio::test]
    async fn a_newer_frame_wins_over_an_older_answer() {
        let cache = RwLock::new(Cached::default());

        // This client's goto(7) is answered over REST...
        assert!(accept(&cache, running(7), 6).await);
        // ...and a move made after it arrives on the socket.
        assert!(accept(&cache, running(2), 7).await);

        assert_eq!(
            cached_slide(&cache).await,
            2,
            "a third party's move was lost"
        );
    }

    /// A restarted server counts from zero, and must not be locked out.
    ///
    /// Without the reset the baseline outlives the connection that issued it,
    /// and every frame the new server sends is numbered below it — so the
    /// client refuses all of them, for good, while looking perfectly healthy.
    #[tokio::test]
    async fn a_dropped_socket_clears_the_baseline() {
        let cache = RwLock::new(Cached::default());

        assert!(accept(&cache, running(9), 12).await);
        // Without this the assertion below fails, which is the whole point.
        cache.write().await.forget_which_change();

        assert!(
            accept(&cache, running(3), 1).await,
            "a restarted server's first change was refused as stale"
        );
        assert_eq!(cached_slide(&cache).await, 3);
    }

    /// A server that numbers nothing behaves as it did before there were
    /// numbers: every frame applied, in arrival order.
    #[tokio::test]
    async fn an_unnumbered_server_still_moves_the_deck() {
        let cache = RwLock::new(Cached::default());

        for slide in [3, 1, 9, 4] {
            assert!(accept(&cache, running(slide), Notification::UNNUMBERED).await);
            assert_eq!(cached_slide(&cache).await, slide);
        }
    }
}
