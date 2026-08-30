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

/// How long to wait between the later asks, once the deck has turned out to be
/// a big one.
const SLOW_RETRY_MS: u32 = 5_000;

/// How many asks at [`RETRY_MS`] before backing off to [`SLOW_RETRY_MS`].
const QUICK_WAITS: u32 = 30;

/// How many times to ask a server that keeps *failing* before giving up.
///
/// At [`RETRY_MS`] this is about two minutes of `500`s, proxy errors or dropped
/// connections — long enough to outlast a restart, short enough not to poll for
/// the length of a talk.
const MAX_FAILURES: u32 = 120;

/// How many times to ask a server that keeps saying "still working".
///
/// Counted apart from [`MAX_FAILURES`], and far larger, because a `503` with a
/// `Retry-After` is a promise rather than a symptom: the server is photographing
/// the deck and saying so. Counting the two together told a speaker with a long
/// deck that previews were *impossible* two minutes in, while the server was
/// still working and about to hand over a complete set — and, because that
/// verdict is terminal, nothing asked again for the rest of the talk.
///
/// At [`QUICK_WAITS`] quick asks and [`SLOW_RETRY_MS`] thereafter this is about
/// a quarter of an hour. Running out is not a verdict: it pauses the chain and
/// leaves [`Readiness::Unknown`] standing, so the message still says the
/// pictures are coming and opening the picker asks again.
const MAX_WAITS: u32 = 200;

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
    /// went on failing until [`MAX_FAILURES`] ran out. Terminal — nothing asks
    /// again until the deck reloads.
    Unavailable,
}

/// What one probe learned.
///
/// Separate from [`Readiness`], which is what a *view* shows: the difference
/// between "still working" and "failed" changes what happens next but not what
/// the speaker reads, and folding the two together is what made a slow deck
/// indistinguishable from a broken one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The pictures are there.
    Ready,
    /// A `503`: the deck is being photographed, and the server asked to be asked
    /// again.
    Working,
    /// A `404`: this deck cannot be photographed at all.
    Impossible,
    /// Anything else — a `500` from a restart, a proxy's `502`, a dropped
    /// connection. Evidence about this request, not about the renderer.
    Failed,
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
    /// `503`s for this version, against [`MAX_WAITS`].
    waits: u32,
    /// Failed asks for this version, against [`MAX_FAILURES`].
    failures: u32,
    /// Whether a chain is already asking about this version.
    ///
    /// One cell is shared by every surface made of photographs, and each of them
    /// has a reason to want a probe started — so without this a page that
    /// reconnected three times ran three chains against one cell, each arming
    /// its own timer and each spending the same budget.
    probing: bool,
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
        self.waits = 0;
        self.failures = 0;
        // Dropping the old timer cancels it, so the previous version's chain
        // does not go on asking beside the new one.
        self.retry = None;
        self.probing = false;
    }

    /// Claims the right to start a probe chain, if there is anything to ask.
    ///
    /// `Some(version)` for the caller that gets it and `None` for everyone else,
    /// so "start asking" is safe to call from anywhere: a first load, a socket
    /// coming back, a picker being opened. The three answers it declines are the
    /// three where a request would be waste — the pictures are here, they never
    /// will be, or someone is already asking.
    ///
    /// The version comes back with the claim rather than being read separately:
    /// a probe that carries the wrong one has its answer thrown away by the
    /// guard in [`probe`], silently.
    pub(crate) fn begin_probe(&mut self) -> Option<u32> {
        match self.readiness {
            Readiness::Ready | Readiness::Unavailable => None,
            Readiness::Unknown if self.probing => None,
            Readiness::Unknown => {
                self.probing = true;
                Some(self.version)
            }
        }
    }
}

/// What one answer means for the chain: what to show, and when to ask again —
/// `None` meaning "do not".
///
/// Pure, and apart from the cell it will be written into, because the whole
/// difference between a deck that is slow and a deck that cannot be
/// photographed is written here, and it is worth being able to test without a
/// browser.
fn verdict(answer: Answer, waits: u32, failures: u32) -> (Readiness, Option<u32>) {
    match answer {
        Answer::Ready => (Readiness::Ready, None),
        Answer::Impossible => (Readiness::Unavailable, None),
        // Out of patience with a server that is nonetheless still working.
        // Pausing, *not* `Unavailable`: the pictures are as possible as they
        // ever were, so the grid goes on saying they are coming and the next
        // time the speaker opens the picker it asks again. Latching a verdict
        // here is what made a long deck indistinguishable from a broken one.
        Answer::Working if waits >= MAX_WAITS => (Readiness::Unknown, None),
        // Long enough to know this is a big deck being photographed rather than
        // one about to appear: back off, so a machine that is already busy is
        // not also being asked once a second while it works.
        Answer::Working if waits > QUICK_WAITS => (Readiness::Unknown, Some(SLOW_RETRY_MS)),
        Answer::Failed if failures >= MAX_FAILURES => (Readiness::Unavailable, None),
        // A deck only just started, or one request that failed: both are worth
        // asking about again, and soon. Which of the two budgets was spent is
        // the caller's business — see the counters above — and not this rule's.
        Answer::Working | Answer::Failed => (Readiness::Unknown, Some(RETRY_MS)),
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
/// [`MAX_FAILURES`] is what keeps retrying them bounded.
///
/// A `503` is not one of those, and is counted apart: it is the server saying it
/// is working. See [`MAX_WAITS`], which paces those asks and, when it runs out,
/// pauses rather than declaring the pictures impossible.
pub(crate) fn probe(thumbs: &Rc<RefCell<Thumbnails>>, version: u32, redraw: Redraw) {
    // Slide 0 stands for all of them: they are made in one pass.
    //
    // `version` is a parameter rather than read from `thumbs`, because every
    // caller is either holding a `RefMut` on it or is a timer armed by one.
    // Nothing here may touch the cell until the `await` below has yielded.
    let url = thumbnail_src(0, version);
    let thumbs = Rc::clone(thumbs);
    spawn_local(async move {
        let answer = match Request::get(&url).send().await {
            Ok(response) if response.ok() => Answer::Ready,
            // Still being photographed. The route says so deliberately, with a
            // `Retry-After`.
            Ok(response) if response.status() == 503 => Answer::Working,
            // The server saying it cannot make them at all.
            Ok(response) if response.status() == 404 => {
                error!(
                    "The server cannot photograph this deck:",
                    url.clone(),
                    "answered 404"
                );
                Answer::Impossible
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
                Answer::Failed
            }
            Err(err) => {
                error!(
                    "Could not reach the server for slide previews:",
                    err.to_string()
                );
                Answer::Failed
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
            match answer {
                Answer::Working => thumbs.waits = thumbs.waits.saturating_add(1),
                Answer::Failed => thumbs.failures = thumbs.failures.saturating_add(1),
                Answer::Ready | Answer::Impossible => {}
            }
            let (readiness, ask_again_in) = verdict(answer, thumbs.waits, thumbs.failures);
            // Said out loud where the chain ends, rather than inside `verdict`,
            // which is a rule and not a reporter.
            match (answer, ask_again_in) {
                (Answer::Working, None) => error!(
                    "Slide previews are still being generated after",
                    MAX_WAITS, "asks — pausing until something asks again"
                ),
                (Answer::Failed, None) => error!(
                    "Giving up on slide previews after",
                    MAX_FAILURES, "failed asks"
                ),
                _ => {}
            }

            thumbs.readiness = readiness;
            thumbs.retry = ask_again_in
                .map(|delay| Timeout::new(delay, move || probe(&handle, version, again)));
            // Nothing is asking any more unless a timer is armed. For `Ready`
            // and `Unavailable` that is moot — `begin_probe` declines both on
            // the verdict alone — but for the paused case it is the whole of
            // what makes asking again possible.
            thumbs.probing = thumbs.retry.is_some();
        }
        redraw();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deck_still_being_photographed_is_never_called_impossible() {
        // The bug this is here for: `503` — the server saying it is working —
        // used to spend the same budget as a dropped connection, so a long deck
        // on a busy machine was declared impossible two minutes in, terminally,
        // while the server was about to hand over a complete set.
        let (readiness, ask_again_in) = verdict(Answer::Working, MAX_WAITS, 0);
        assert_ne!(readiness, Readiness::Unavailable);
        assert_eq!(readiness, Readiness::Unknown);
        // Paused rather than settled, which is what leaves it resumable.
        assert_eq!(ask_again_in, None);
    }

    #[test]
    fn waiting_does_not_spend_the_failure_budget() {
        let (readiness, ask_again_in) = verdict(Answer::Working, MAX_FAILURES, 0);
        assert_eq!(readiness, Readiness::Unknown);
        assert!(ask_again_in.is_some());
    }

    #[test]
    fn a_server_that_keeps_failing_is_given_up_on() {
        assert_eq!(
            verdict(Answer::Failed, 0, MAX_FAILURES),
            (Readiness::Unavailable, None)
        );
        // One failure is evidence about one request, and worth repeating.
        assert_eq!(
            verdict(Answer::Failed, 0, 1),
            (Readiness::Unknown, Some(RETRY_MS))
        );
    }

    #[test]
    fn only_a_404_is_terminal_at_once() {
        assert_eq!(
            verdict(Answer::Impossible, 0, 0),
            (Readiness::Unavailable, None)
        );
        assert_eq!(verdict(Answer::Ready, 0, 0), (Readiness::Ready, None));
    }

    #[test]
    fn asking_backs_off_once_the_deck_turns_out_to_be_a_long_one() {
        assert_eq!(verdict(Answer::Working, 1, 0).1, Some(RETRY_MS));
        assert_eq!(
            verdict(Answer::Working, QUICK_WAITS + 1, 0).1,
            Some(SLOW_RETRY_MS)
        );
    }

    #[test]
    fn only_one_chain_asks_at_a_time() {
        // What a reconnect used to cost: `fetch_talk_metadata` runs on every
        // `Connected`, so three blips of a room's wifi put three probe chains on
        // one cell, each arming its own timer and each spending the same budget.
        let mut thumbs = Thumbnails::default();
        assert_eq!(thumbs.begin_probe(), Some(0));
        assert_eq!(thumbs.begin_probe(), None);
        assert_eq!(thumbs.begin_probe(), None);
    }

    #[test]
    fn a_reload_is_asked_about_again_and_a_verdict_is_not() {
        let mut thumbs = Thumbnails::default();
        thumbs.begin_probe();

        // A deck edited under the speaker: new pictures at the same URLs.
        thumbs.invalidate();
        assert_eq!(thumbs.begin_probe(), Some(1));

        // An answer, on the other hand, is not worth asking for twice.
        thumbs.readiness = Readiness::Ready;
        assert_eq!(thumbs.begin_probe(), None);
        thumbs.readiness = Readiness::Unavailable;
        assert_eq!(thumbs.begin_probe(), None);
    }

    #[test]
    fn a_paused_wait_is_resumable() {
        // What `probe` leaves behind when it runs out of patience with a server
        // that is still working: no verdict, and nothing asking — so the next
        // thing that wants a picture starts the chain again.
        let mut thumbs = Thumbnails::default();
        thumbs.begin_probe();
        thumbs.probing = false;

        assert_eq!(thumbs.readiness, Readiness::Unknown);
        assert_eq!(thumbs.begin_probe(), Some(0));
    }
}
