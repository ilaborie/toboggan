//! Whether the deck's photographs exist yet, and asking again until they do.
//!
//! Two surfaces of the presenter view are pictures of the deck rather than
//! renderings of it — the next-slide pane and the slide picker's grid — and both
//! read the same two facts: are the thumbnails ready, and which generation of
//! them is current. The state lives on [`super::Inner`] because the pane wants
//! it while the picker is closed; it used to belong to the picker, which stopped
//! asking the moment it was shut.
//!
//! **Thumbnails may not exist yet.** The server photographs the deck as it
//! starts, and on the first request that wants a picture when
//! `--no-eager-thumbnails` says otherwise — either way it takes seconds. An
//! `<img>` pointed at a thumbnail that is not ready gets a `503` and stays
//! broken for ever, so this asks over `fetch`, where it can tell "still
//! working" (retry) from "never going to work" (say so and stop).

use std::cell::RefCell;
use std::rc::Rc;

use gloo::console::error;
use gloo::net::http::Request;
use gloo::timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;

use super::Inner;

/// How long to wait before asking again whether the thumbnails are ready.
///
/// The same order as the generation itself — a few seconds for a deck — so a
/// speaker who opens the view on a cold server sees it fill in rather than
/// watching a spinner that polls twenty times a second.
const RETRY_MS: u32 = 1_000;

/// How many times to ask before giving up.
///
/// At [`RETRY_MS`] this is about two minutes. A deck takes seconds to
/// photograph, so anything still answering "not yet" this long afterwards is a
/// server that is not going to — and a view that polls for ever is a view that
/// is lying about being about to work.
const MAX_ATTEMPTS: u32 = 120;

/// Whether the deck's photographs can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Readiness {
    /// Not asked yet, or asking again.
    #[default]
    Unknown,
    /// The server has thumbnails; the pictures carry real `src`s.
    Ready,
    /// There are no pictures to be had: the server said so with a `404`, or it
    /// went on failing until [`MAX_ATTEMPTS`] ran out. Terminal — nothing asks
    /// again until the deck reloads.
    Unavailable,
}

/// What the presenter view believes about the deck's photographs.
#[derive(Default)]
pub(super) struct Thumbnails {
    /// Bumped on every reload, and appended to each `src`.
    ///
    /// The thumbnail of slide 4 is a *different picture* at the same URL after
    /// the deck reloads, and the route says `no-cache` — but a cache is not the
    /// only thing that would hold the old one: an `<img>` whose `src` is set to
    /// the string it already has does not re-fetch at all.
    pub(super) version: u32,
    pub(super) readiness: Readiness,
    /// Held so it stays armed; dropped to stop asking.
    pub(super) retry: Option<Timeout>,
    /// Probes sent for this version, against [`MAX_ATTEMPTS`].
    attempts: u32,
}

impl Thumbnails {
    /// The deck changed under the speaker: every picture is now potentially a
    /// different picture, and the server is making them again.
    ///
    /// A failed deck may render on the next save, so a reload is also the one
    /// event that earns a retry after [`Readiness::Unavailable`].
    pub(super) fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.readiness = Readiness::Unknown;
        self.attempts = 0;
        // Dropping the old timer cancels it, so the previous version's chain
        // does not go on asking beside the new one.
        self.retry = None;
    }
}

/// Where one presented slide's picture lives.
///
/// A *presented* index, not an authored one: the server crosses between the two
/// index spaces behind this route, so nothing here has to know the deck hides
/// slides from the web. The version is a cache-buster; the route ignores it.
pub(super) fn thumbnail_src(index: usize, version: u32) -> String {
    format!("/overview/slide/{index}?v={version}")
}

/// Asks the server whether the thumbnails are ready, and keeps asking while they
/// are merely late.
///
/// `fetch` rather than an `<img>`'s `error` event, because the two answers that
/// matter are indistinguishable to an image: `503` means the deck is still being
/// photographed and this is worth repeating, `404` means it never will be and
/// repeating it is a lie told once a second for the length of the talk.
///
/// Only `404` is terminal. A `500` from a server mid-restart, a `502` from a
/// proxy, or the `Err` a dropped connection produces are all evidence about
/// *this request*, not about whether the machine can photograph a deck — and
/// latching on one of them told a speaker their previews were impossible for
/// the rest of the talk while the server sat on a complete set of PNGs.
/// [`MAX_ATTEMPTS`] is what keeps retrying them bounded.
pub(super) fn probe(inner: Rc<RefCell<Inner>>, version: u32) {
    // Slide 0 stands for all of them: they are made in one pass.
    //
    // `version` is a parameter rather than read from `inner`, because every
    // caller is either holding a `RefMut` on it or is a timer armed by one.
    // Nothing here may touch the cell until the `await` below has yielded.
    let url = thumbnail_src(0, version);
    spawn_local(async move {
        let readiness = match Request::get(&url).send().await {
            Ok(response) if response.ok() => Readiness::Ready,
            // Still being photographed. The route says so deliberately, with a
            // `Retry-After`.
            Ok(response) if response.status() == 503 => Readiness::Unknown,
            // The server saying it cannot make them at all.
            Ok(response) if response.status() == 404 => {
                error!(
                    "The server cannot photograph this deck:",
                    url.clone(),
                    "answered 404"
                );
                Readiness::Unavailable
            }
            // Anything else is not an answer about the renderer, so it is worth
            // asking again — but it is worth saying out loud too, because the
            // status is the only clue to why the grid is empty.
            Ok(response) => {
                error!(
                    "Unexpected status probing slide previews:",
                    response.status(),
                    url.clone()
                );
                Readiness::Unknown
            }
            Err(err) => {
                error!(
                    "Could not reach the server for slide previews:",
                    err.to_string()
                );
                Readiness::Unknown
            }
        };

        let handle = Rc::clone(&inner);
        let mut inner = inner.borrow_mut();
        // A reload landed while the request was in flight, and this answer is
        // about the previous set of pictures — which were ready, while the ones
        // now being made are not. Believing it would put broken images in the
        // grid and stop anything asking again.
        if inner.thumbs.version != version {
            return;
        }
        inner.thumbs.attempts = inner.thumbs.attempts.saturating_add(1);
        let exhausted = inner.thumbs.attempts >= MAX_ATTEMPTS;
        // Out of patience: settle rather than go on asking for the length of
        // the talk. The message the picker shows says only that there are no
        // previews, which is true whichever way we got here.
        let readiness = match (readiness, exhausted) {
            (Readiness::Unknown, true) => {
                error!(
                    "Giving up on slide previews after",
                    MAX_ATTEMPTS, "attempts"
                );
                Readiness::Unavailable
            }
            (readiness, _) => readiness,
        };

        inner.thumbs.readiness = readiness;
        inner.refresh_thumbnails();
        match readiness {
            Readiness::Ready | Readiness::Unavailable => inner.thumbs.retry = None,
            Readiness::Unknown => {
                inner.thumbs.retry = Some(Timeout::new(RETRY_MS, move || probe(handle, version)));
            }
        }
    });
}
