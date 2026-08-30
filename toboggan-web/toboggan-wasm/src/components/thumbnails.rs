//! Whether the deck's photographs exist yet, and asking again until they do.
//!
//! Several surfaces are pictures of the deck rather than renderings of it — the
//! presenter view's next-slide pane and the slide picker's grid — and they all
//! read the same two facts: are the thumbnails ready, and which generation of
//! them is current. One [`Thumbnails`] is shared between them, so a verdict
//! reached for one is a verdict for all: the state used to belong to the picker,
//! which stopped asking the moment it was shut.
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

/// What a surface made of photographs does when the verdict changes.
///
/// Shared rather than owned, because [`probe`] hands it to the timer that
/// re-runs it. It is a callback rather than a handle to any one view: the two
/// surfaces that read a [`Thumbnails`] live in different components, and the
/// probe has no business knowing which of them it is redrawing.
pub(crate) type Redraw = Rc<dyn Fn()>;

/// Whether the deck's photographs can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Readiness {
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

/// What a view believes about the deck's photographs.
#[derive(Default)]
pub(crate) struct Thumbnails {
    /// Bumped on every reload, and appended to each `src`.
    ///
    /// The thumbnail of slide 4 is a *different picture* at the same URL after
    /// the deck reloads, and the route says `no-cache` — but a cache is not the
    /// only thing that would hold the old one: an `<img>` whose `src` is set to
    /// the string it already has does not re-fetch at all.
    version: u32,
    readiness: Readiness,
    /// Held so it stays armed; dropped to stop asking.
    retry: Option<Timeout>,
    /// Probes sent for this version, against [`MAX_ATTEMPTS`].
    attempts: u32,
}

impl Thumbnails {
    pub(crate) fn readiness(&self) -> Readiness {
        self.readiness
    }

    pub(crate) fn version(&self) -> u32 {
        self.version
    }

    /// The deck changed under the speaker: every picture is now potentially a
    /// different picture, and the server is making them again.
    ///
    /// A failed deck may render on the next save, so a reload is also the one
    /// event that earns a retry after [`Readiness::Unavailable`].
    pub(crate) fn invalidate(&mut self) {
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
pub(crate) fn thumbnail_src(index: usize, version: u32) -> String {
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
pub(crate) fn probe(thumbs: &Rc<RefCell<Thumbnails>>, version: u32, redraw: Redraw) {
    // Slide 0 stands for all of them: they are made in one pass.
    //
    // `version` is a parameter rather than read from `thumbs`, because every
    // caller is either holding a `RefMut` on it or is a timer armed by one.
    // Nothing here may touch the cell until the `await` below has yielded.
    let url = thumbnail_src(0, version);
    let thumbs = Rc::clone(thumbs);
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

        // Scoped, because `redraw` is what reads this cell back out: calling it
        // with the borrow still held is a panic the moment a view repaints from
        // the verdict this just wrote.
        {
            let handle = Rc::clone(&thumbs);
            let again = Rc::clone(&redraw);
            let mut thumbs = thumbs.borrow_mut();
            // A reload landed while the request was in flight, and this answer
            // is about the previous set of pictures — which were ready, while
            // the ones now being made are not. Believing it would put broken
            // images in the grid and stop anything asking again.
            if thumbs.version != version {
                return;
            }
            thumbs.attempts = thumbs.attempts.saturating_add(1);
            let exhausted = thumbs.attempts >= MAX_ATTEMPTS;
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

            thumbs.readiness = readiness;
            thumbs.retry = match readiness {
                Readiness::Ready | Readiness::Unavailable => None,
                Readiness::Unknown => Some(Timeout::new(RETRY_MS, move || {
                    probe(&handle, version, again);
                })),
            };
        }
        redraw();
    });
}
