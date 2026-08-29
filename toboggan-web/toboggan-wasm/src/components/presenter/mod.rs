//! The presenter view: what the speaker looks at while the room looks at the
//! deck.
//!
//! The current-slide pane is an `<iframe>` of the deck itself, painted by
//! `postMessage` from here — see [`crate::services::mirror`]. It used to be a
//! second rendering of the slide component, scaled with CSS `zoom`, and that
//! could not be made faithful: the slide inherited this shadow tree's 16px base
//! instead of the deck's viewport-derived one, the zoom factors were tuned for a
//! single window size, there was no footer, and the deck's own `_head.html` —
//! arbitrary author CSS, injected into *this* document — restyled the speaker's
//! chrome along with the slide.
//!
//! The next-slide pane is not a mirror at all but a *photograph*: the same
//! `/overview/slide/{index}` still the picker's grid shows. A second live deck
//! in an iframe cost a whole wasm client, a slide fetch per move and a mirror
//! handshake, to draw a picture the size of a stamp that nobody interacts with.
//!
//! The view drives the deck two ways: the keyboard the deck already provides,
//! and the buttons in the status strip. Both write to the one channel
//! `handle_actions` reads, so a client that may not present is refused, told
//! why, and logged in exactly one place.

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::error;
use gloo::events::EventListener;
use gloo::timers::callback::Interval;
use gloo::utils::window;
use toboggan_core::{Command, Slide, SlideId, SlideOutline, State, TalkResponse};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;
use web_sys::{
    Element, HtmlDialogElement, HtmlElement, HtmlIFrameElement, HtmlInputElement, MessageEvent,
    ResizeObserver,
};

use crate::components::WasmElement;
use crate::services::mirror::{self, MirrorFrame, MirrorMessage, MirrorPane};
use crate::{
    StateClassMapper as _, create_and_append_element, create_shadow_root_with_style, dom_try,
    render_content,
};

mod picker;
mod thumbnails;
use self::picker::Picker;
use self::thumbnails::{Readiness, Thumbnails, thumbnail_src};

const CSS: &str = include_str!("style.css");

/// How often the clock and the elapsed timer redraw.
const TICK_MS: u32 = 1_000;

/// The logical viewport every mirror is laid out in, whatever size its pane is.
///
/// Fixed, and then shrunk with a transform, because a deck sizes itself against
/// its viewport: `main.css` derives the root font size from `min(1.667vw,
/// 2.963vh)`, so an iframe laid out at its real pane width would pick a smaller
/// root and break its lines somewhere the projector does not. A transform
/// changes nothing about layout — only the paint — so the pane is a true
/// reduction of a 1280×720 screen.
const STAGE_WIDTH: f64 = 1280.0;

/// Where the speaker's notes size is remembered between talks.
const NOTES_SIZE_KEY: &str = "toboggan.presenter.notes-size";

/// The range the notes may be sized within, in pixels, and the step the buttons
/// move by. Bounded because the control is two buttons held down, not a slider.
const NOTES_SIZE_RANGE: (f64, f64) = (14.0, 34.0);
const NOTES_SIZE_STEP: f64 = 2.0;
const NOTES_SIZE_DEFAULT: f64 = 20.0;

/// The bits of the status strip that change.
struct StatusBar {
    clock: Element,
    elapsed: Element,
    pause: Element,
    progress: Element,
    counter: Element,
    steps: Element,
    pacing: Element,
}

/// One of the two panes: its iframe, and what has been posted into it.
struct Stage {
    frame: HtmlIFrameElement,
    /// The last frame built for this pane, re-posted whenever its mirror says it
    /// has come up. The mirrors finish loading well after the socket has started
    /// delivering state, so without this a pane shows whatever slide the deck
    /// was on when the presenter view opened.
    last: Option<MirrorFrame>,
    /// Whether the mirror has announced itself; see [`MirrorMessage::Ready`].
    ready: bool,
}

impl Stage {
    /// Remembers `frame`, and posts it if the mirror is listening.
    fn show(&mut self, frame: MirrorFrame) {
        self.last = Some(frame);
        self.post();
    }

    /// Re-posts whatever this pane was last given.
    fn post(&self) {
        if !self.ready {
            return;
        }
        let (Some(frame), Some(origin)) = (self.last.as_ref(), mirror::page_origin()) else {
            return;
        };
        let Some(message) = mirror::encode(&MirrorMessage::Frame(Box::new(frame.clone()))) else {
            return;
        };
        let Some(target) = self.frame.content_window() else {
            error!("A presenter pane has no window to paint into");
            return;
        };
        if let Err(err) = target.post_message(&message, &origin) {
            error!("A presenter pane refused a frame:", err);
        }
    }
}

/// The talk's elapsed time, as a running origin plus whatever was banked before
/// the last pause.
///
/// Two fields rather than the single wall-clock origin this replaces, because a
/// paused clock has no origin to measure from and a running one has no total to
/// add up. `started` is the third thing neither says: a clock paused at zero and
/// a clock that has never run look identical, and only one of them should start
/// itself when the deck is first seen running.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Elapsed {
    started: bool,
    running_since: Option<f64>,
    banked: f64,
}

impl Elapsed {
    /// Starts the clock the first time the deck is seen running, so opening the
    /// view before the talk does not have it counting the coffee break.
    fn start_if_idle(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        self.running_since = Some(js_sys::Date::now());
    }

    fn toggle(&mut self) {
        self.started = true;
        match self.running_since {
            Some(since) => {
                self.banked += (js_sys::Date::now() - since).max(0.0);
                self.running_since = None;
            }
            None => self.running_since = Some(js_sys::Date::now()),
        }
    }

    /// Back to zero, in whatever run state it was already in — a reset while
    /// paused should not silently start the talk.
    fn restart(&mut self) {
        self.banked = 0.0;
        if self.running_since.is_some() {
            self.running_since = Some(js_sys::Date::now());
        }
    }

    const fn is_running(&self) -> bool {
        self.running_since.is_some()
    }

    fn secs(&self) -> u64 {
        let live = self
            .running_since
            .map_or(0.0, |since| (js_sys::Date::now() - since).max(0.0));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let secs = ((self.banked + live) / 1_000.0) as u64;
        secs
    }
}

/// The parts the ticker, the buttons and the state updates all touch.
struct Inner {
    /// Carries `data-role`, so the stylesheet decides what an audience client is
    /// shown rather than this file setting inline styles.
    layout: Element,
    now: Stage,
    notes: Option<Element>,
    /// The next slide's photograph, `/overview/slide/{index + 1}`.
    next_shot: Option<Element>,
    next_title: Option<Element>,
    next_number: Option<Element>,
    status: Option<StatusBar>,
    /// The whole deck at a glance, searchable. Shared rather than owned, because
    /// its click handlers outlive any one call and have to reach it.
    picker: Option<Rc<RefCell<Picker>>>,
    /// Whether the deck's photographs exist yet, and which generation of them.
    /// Read by both the next pane and the picker, which is why it is here and
    /// not in either.
    thumbs: Thumbnails,

    // Everything from `GET /api/talk`. The markup rides on every frame, which is
    // what makes a `TalkChange` that edits only `_head.html` reach both mirrors.
    head: Option<String>,
    footer: Option<String>,
    lang: Option<String>,
    /// Planned seconds per slide, from the deck's `duration` front matter.
    /// Empty when the deck plans nothing, which hides the pacing readout.
    plan: Vec<Option<u64>>,
    /// Reveal steps per slide, so the counter can say `2/3` rather than `2`.
    step_counts: Vec<usize>,
    /// Every slide's title, as `GET /api/talk` gives them: `Content::display_text`,
    /// which is the bare text for the `Content::Text` every parser path builds a
    /// title as, and an HTML title's raw markup otherwise. Treated as text
    /// either way — see [`Inner::refresh_next`].
    ///
    /// The next pane's title is read from here rather than fetched: it is the
    /// only part of the next slide this view still needs, now that the pane is
    /// a photograph.
    titles: Vec<String>,
    total_slides: usize,

    // The deck's position and the two slides around it, kept so a frame can be
    // rebuilt without being handed one — which a mirror's handshake and a live
    // reload both need.
    state_class: String,
    current_index: usize,
    current_step: usize,
    current_slide: Option<Slide>,
    /// Whether the deck is on a slide at all. `current_index` cannot say: before
    /// the talk starts and past the last slide it is 0, which is also slide one.
    on_slide: bool,

    elapsed: Elapsed,
}

#[derive(Default)]
pub(crate) struct TobogganPresenterElement {
    inner: Option<Rc<RefCell<Inner>>>,
    /// Where the on-screen buttons send their commands. Set before `render`,
    /// which is where the buttons are wired.
    commands: Option<UnboundedSender<Command>>,
    /// Held only to keep them alive for the page's lifetime — dropping the
    /// interval stops the clock, dropping a listener stops its button, dropping
    /// the inbox stops the panes updating.
    ticker: Option<Interval>,
    listeners: Vec<EventListener>,
    inbox: Option<EventListener>,
    scale: Option<ResizeObserver>,
    /// Kept beside the observer because a `ResizeObserver` is held alive by the
    /// elements it watches rather than by this handle, so freeing the closure
    /// while it is still observing leaves the browser calling into memory that
    /// has gone.
    scale_callback: Option<Closure<dyn FnMut()>>,
}

impl Drop for TobogganPresenterElement {
    fn drop(&mut self) {
        if let Some(observer) = &self.scale {
            observer.disconnect();
        }
    }
}

impl TobogganPresenterElement {
    /// Points the on-screen navigation at the same channel the keyboard writes
    /// to. Must be called before [`WasmElement::render`].
    pub(crate) fn set_commands(&mut self, commands: UnboundedSender<Command>) {
        self.commands = Some(commands);
    }

    /// Records the deck's shape and the markup every pane needs: everything from
    /// `GET /api/talk` that either the status strip or a mirror reads.
    ///
    /// Re-posts both frames, so a live reload that edits only `_footer.html` or
    /// `_head.html` — with the deck sitting on the same slide, so no state
    /// change follows it — still reaches the panes.
    pub(crate) fn set_talk(&self, talk: &TalkResponse) {
        let Some(inner) = &self.inner else {
            return;
        };
        let version = {
            let mut inner = inner.borrow_mut();
            inner.total_slides = talk.titles.len();
            inner.titles.clone_from(&talk.titles);
            // Every reload lands here, including the ones that only edited a
            // slide: the deck may be a different length, and every thumbnail is
            // potentially a different picture at the same URL.
            inner.thumbs.invalidate();
            if let Some(picker) = &inner.picker {
                let mut picker = picker.borrow_mut();
                // The corpus describes the deck that just went away. `app.rs`
                // asks for the new one straight after this, and if that request
                // fails the picker is left unsearchable rather than searching
                // the old deck's words against the new deck's cells.
                picker.forget_outline();
                picker.build(talk.titles.len());
            }
            inner.plan.clone_from(&talk.durations);
            inner.step_counts.clone_from(&talk.step_counts);
            inner.head.clone_from(&talk.head);
            inner.footer.clone_from(&talk.footer);
            inner.lang.clone_from(&talk.lang);
            inner.refresh_status();
            inner.refresh_next();
            inner.refresh_thumbnails();
            inner.post_now();
            inner.thumbs.version
        };
        // Outside the borrow: the probe takes the handle, and it borrows.
        thumbnails::probe(Rc::clone(inner), version);
    }

    /// Takes the deck as plain text, so the picker can be searched.
    ///
    /// Optional in the sense that everything still works without it: the picker
    /// opens, shows the deck and jumps: only the filtering is reduced to nothing
    /// matching more than the whole deck.
    pub(crate) fn set_outline(&self, slides: &[SlideOutline]) {
        let Some(inner) = &self.inner else {
            return;
        };
        let inner = inner.borrow();
        if let Some(picker) = &inner.picker {
            picker.borrow_mut().set_outline(slides);
        }
    }

    /// Takes the deck's position and the slide it is on, and redraws everything
    /// derived from them: the notes, the status strip, and the "now" pane.
    pub(crate) fn set_state(&self, state: &State, slide: Option<Slide>) {
        let Some(inner) = &self.inner else {
            return;
        };
        let mut inner = inner.borrow_mut();

        if state.current().is_some() {
            inner.elapsed.start_if_idle();
        }
        inner.on_slide = state.current().is_some();
        inner.current_index = state.current().map_or(0, SlideId::index);
        inner.current_step = state.current_step();
        state.to_css_class().clone_into(&mut inner.state_class);

        let notes = slide
            .as_ref()
            .map(|slide| render_content(&slide.notes, None));
        inner.current_slide = slide;
        if let Some(element) = &inner.notes {
            element.set_inner_html(notes.as_deref().unwrap_or_default());
        }

        if let Some(picker) = &inner.picker {
            picker.borrow_mut().set_current(inner.current_index);
        }

        inner.refresh_steps(state);
        inner.refresh_status();
        inner.refresh_next();
        inner.post_now();
    }

    /// Shows or hides the navigation, matching the "Following along" toast: a
    /// button that does nothing is worse than no button.
    pub(crate) fn set_can_drive(&self, can_drive: bool) {
        let Some(inner) = &self.inner else {
            return;
        };
        let role = if can_drive { "presenter" } else { "audience" };
        let _ = inner.borrow().layout.set_attribute("data-role", role);
    }
}

impl Inner {
    /// Builds the frame for one pane out of what the deck last said.
    fn frame_for(&self, pane: MirrorPane) -> MirrorFrame {
        let shared = |slide, step, state_class, slide_number| MirrorFrame {
            head: self.head.clone(),
            footer: self.footer.clone(),
            lang: self.lang.clone(),
            slide,
            step,
            state_class,
            slide_number,
            total_slides: self.total_slides,
        };
        match pane {
            MirrorPane::Current => shared(
                self.current_slide.clone(),
                Some(self.current_step),
                self.state_class.clone(),
                self.current_index + 1,
            ),
        }
    }

    fn post_now(&mut self) {
        let frame = self.frame_for(MirrorPane::Current);
        self.now.show(frame);
    }

    /// Answers a mirror that has just come up.
    ///
    /// Rebuilt rather than replayed: a mirror that reloaded mid-talk should come
    /// back to where the deck is now, not to the frame it died on.
    fn welcome(&mut self, pane: MirrorPane) {
        match pane {
            MirrorPane::Current => {
                self.now.ready = true;
                self.post_now();
            }
        }
    }

    /// Which slide the next pane is showing, or `None` at either end of the
    /// deck — before it starts, and past its last slide.
    fn next_index(&self) -> Option<usize> {
        let next = self.current_index.checked_add(1)?;
        (self.on_slide && next < self.total_slides).then_some(next)
    }

    /// Redraws the next-slide pane: its number, its title, and its photograph.
    fn refresh_next(&self) {
        let next = self.next_index();
        if let Some(element) = &self.next_number {
            // 1-based, the way every other number the speaker reads is.
            let text = next
                .map(|index| format!("{}", index + 1))
                .unwrap_or_default();
            element.set_text_content(Some(&text));
        }
        if let Some(element) = &self.next_title {
            let title = next.and_then(|index| self.titles.get(index));
            // Text, never markup. A title is author input, and this pane is the
            // speaker's chrome: `render_content` — which is what draws a title
            // on the deck itself — escapes it, and reading it out of
            // `TalkResponse::titles` instead does not make it safe to set as
            // HTML. Same reasoning as the picker's captions and snippets.
            element.set_text_content(title.map(String::as_str));
        }
        let Some(image) = &self.next_shot else {
            return;
        };
        match (next, self.thumbs.readiness) {
            (Some(index), Readiness::Ready) => {
                let _ = image.set_attribute("src", &thumbnail_src(index, self.thumbs.version));
            }
            // No photograph to show: the end of the deck, a machine that cannot
            // make them, or a deck still being photographed. The pane keeps its
            // number and its title, and `.next-shot:not([src])` hides the broken
            // image the browser would otherwise draw.
            _ => {
                let _ = image.remove_attribute("src");
            }
        }
    }

    /// Redraws everything that is a photograph of the deck.
    fn refresh_thumbnails(&mut self) {
        self.refresh_next();
        if let Some(picker) = &self.picker {
            picker
                .borrow_mut()
                .refresh(self.thumbs.readiness, self.thumbs.version);
        }
    }

    /// Redraws the reveal counter for the slide the deck is on.
    ///
    /// Blank on a slide with no reveals rather than `0/0`, which reads as
    /// something being wrong on the two thirds of slides that have none.
    fn refresh_steps(&self, state: &State) {
        let Some(status) = &self.status else {
            return;
        };
        let total = self.step_counts.get(self.current_index).copied();
        let text = match total {
            Some(total) if total > 0 => format!("step {}/{total}", state.current_step()),
            _ => String::new(),
        };
        status.steps.set_text_content(Some(&text));
    }

    /// Redraws the clock, the timer, the counters and the pacing.
    fn refresh_status(&self) {
        let Some(status) = &self.status else {
            return;
        };
        let now = js_sys::Date::new_0();
        status.clock.set_text_content(Some(&format!(
            "{:02}:{:02}",
            now.get_hours(),
            now.get_minutes()
        )));

        let elapsed = self.elapsed.secs();
        status
            .elapsed
            .set_text_content(Some(&format!("⏱ {}", format_duration(elapsed))));

        let running = self.elapsed.is_running();
        status
            .pause
            .set_text_content(Some(if running { "⏸" } else { "▶" }));
        let _ = status
            .pause
            .set_attribute("aria-pressed", if running { "false" } else { "true" });
        let _ = status.pause.set_attribute(
            "title",
            if running {
                "Pause the timer"
            } else {
                "Resume the timer"
            },
        );

        let displayed = self.current_index + 1;
        status.counter.set_inner_html(&format!(
            "<b>{displayed}</b>/{total}",
            total = self.total_slides
        ));

        let fraction = if self.total_slides == 0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let done = displayed.min(self.total_slides) as f64;
            #[allow(clippy::cast_precision_loss)]
            let total = self.total_slides as f64;
            done / total
        };
        if let Some(bar) = status.progress.first_element_child()
            && let Ok(bar) = bar.dyn_into::<HtmlElement>()
        {
            let _ = bar
                .style()
                .set_property("inline-size", &format!("{:.1}%", fraction * 100.0));
        }

        if let Some(drift) = self.drift_secs(elapsed) {
            let (pace, sign) = if drift < 0 {
                ("early", '−')
            } else {
                ("late", '+')
            };
            let _ = status.pacing.set_attribute("data-pace", pace);
            status.pacing.set_text_content(Some(&format!(
                "{sign}{}",
                format_duration(drift.unsigned_abs())
            )));
        } else {
            let _ = status.pacing.remove_attribute("data-pace");
            status.pacing.set_text_content(None);
        }
    }

    /// Seconds ahead (negative) or behind (positive) the deck's own plan.
    ///
    /// Measured against the time the *previous* slides were meant to take, so
    /// it answers "should I already have been here?" — the question a speaker
    /// glancing down mid-sentence is actually asking. `None` when the deck
    /// declares no durations, in which case there is no plan to be late for.
    fn drift_secs(&self, elapsed: u64) -> Option<i64> {
        if self.plan.iter().all(Option::is_none) {
            return None;
        }
        let planned_before = self
            .plan
            .iter()
            .take(self.current_index)
            .filter_map(|planned| *planned)
            .sum::<u64>();
        Some(i64::try_from(elapsed).ok()? - i64::try_from(planned_before).ok()?)
    }
}

/// `mm:ss`, growing to `h:mm:ss` once a talk runs past the hour.
fn format_duration(secs: u64) -> String {
    let (hours, minutes, seconds) = (secs / 3_600, (secs % 3_600) / 60, secs % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Where a mirror of the deck is served from, as seen from this page.
///
/// `/presenter` is the server's route for this page and `/run` is the deck's;
/// under `pnpm dev` vite serves the files themselves — `presenter.html` and
/// `index.html` — and has no `/run` route at all. Deriving the pair from this
/// page's own path is what keeps the development server working.
fn mirror_src(pane: MirrorPane) -> String {
    let path = window()
        .location()
        .pathname()
        .unwrap_or_else(|_| "/presenter".to_owned());
    // `/presenter` is the server's route for this page and `/run` is the deck's.
    // Anything else is `pnpm dev` serving the files themselves — `presenter.html`
    // beside `index.html` — where there is no `/run` route to ask for.
    let deck = if path == "/presenter" {
        "/run"
    } else {
        "index.html"
    };
    format!("{deck}?mirror={}", pane.as_str())
}

/// The speaker's chrome, as one string.
///
/// Out of line from `render` only because it is markup: reading it beside the
/// wiring made both harder to follow, and every selector below is matched
/// against it.
fn layout_html() -> String {
    format!(
        r#"<div class="now"><div class="pane screen"><iframe class="mirror" title="Current slide" src="{now}"></iframe></div></div>
<div class="aside">
  <p class="label">Next</p>
  <div class="next"><div class="pane shot"><img class="next-shot" alt="The next slide"></div></div>
  <p class="next-meta"><span class="next-number"></span><span class="next-title"></span></p>
  <div class="notes-head">
    <span class="label">Notes</span>
    <span class="notes-size">
      <button type="button" class="notes-smaller" title="Smaller notes" aria-label="Smaller notes">A−</button>
      <button type="button" class="notes-bigger" title="Bigger notes" aria-label="Bigger notes">A+</button>
    </span>
  </div>
  <div class="pane notes"></div>
</div>
<div class="status">
  <span class="clock"></span>
  <span class="timer">
    <span class="elapsed"></span>
    <button type="button" class="pause" aria-pressed="false" title="Pause the timer">⏸</button>
    <button type="button" class="reset" title="Restart the timer" aria-label="Restart the timer">↺</button>
  </span>
  <span class="progress"><span></span></span>
  <span class="counter"></span>
  <span class="steps"></span>
  <span class="pacing"></span>
  <nav class="nav">
    <button type="button" class="strip-toggle" title="Find a slide (g, / or Ctrl+K)" aria-label="Find a slide" aria-expanded="false">▦</button>
    <button type="button" class="go-prev" title="Previous step" aria-label="Previous step">‹</button>
    <button type="button" class="go-next" title="Next step" aria-label="Next step">›</button>
  </nav>
</div>
<dialog class="picker">
  <div class="strip-head">
    <input class="strip-search" type="search" placeholder="Find a slide…" aria-label="Find a slide"
           autocomplete="off" autofocus role="combobox" aria-expanded="true" aria-controls="slide-grid">
    <span class="strip-count"></span>
    <span class="strip-status"></span>
    <button type="button" class="strip-close" title="Close (Esc)" aria-label="Close the slide picker">✕</button>
  </div>
  <div class="strip-grid" id="slide-grid" role="listbox" aria-label="Slides"></div>
  <p class="strip-hint">type to filter · ↑↓←→ move · ⏎ go · Esc close</p>
</dialog>"#,
        now = mirror_src(MirrorPane::Current),
    )
}

/// Collects the parts of the status strip that change, or `None` if any is
/// missing — the strip is redrawn as a whole, and half of one is worse than
/// none.
fn find_status(find: &impl Fn(&str) -> Option<Element>) -> Option<StatusBar> {
    match (
        find(".clock"),
        find(".elapsed"),
        find(".pause"),
        find(".progress"),
        find(".counter"),
        find(".steps"),
        find(".pacing"),
    ) {
        (
            Some(clock),
            Some(elapsed),
            Some(pause),
            Some(progress),
            Some(counter),
            Some(steps),
            Some(pacing),
        ) => Some(StatusBar {
            clock,
            elapsed,
            pause,
            progress,
            counter,
            steps,
            pacing,
        }),
        _ => None,
    }
}

impl WasmElement for TobogganPresenterElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create presenter shadow root"
        );

        let layout: Element = dom_try!(create_and_append_element(&root, "div"), "presenter layout");
        layout.set_class_name("layout");
        // Starts as `presenter`, matching the client's own default role: on an
        // ordinary local deck the handshake confirms it rather than correcting
        // it, and starting hidden would flash the controls away and back.
        let _ = layout.set_attribute("data-role", "presenter");
        layout.set_inner_html(&layout_html());

        // Each miss is named. This used to be `.ok().flatten()`, which folded a
        // malformed selector and an absent element into the same `None`, and
        // the six below were then matched as a tuple — so one missing element
        // blanked the clock, the timer, the progress bar, the slide counter,
        // the step counter and the pacing together, with nothing in the
        // console. Every selector here is matched against markup written a
        // hundred lines above, so it fires exactly when someone edits that.
        let find = |selector: &str| match layout.query_selector(selector) {
            Ok(Some(element)) => Some(element),
            Ok(None) => {
                error!("Presenter layout is missing an element:", selector);
                None
            }
            Err(err) => {
                error!("Presenter layout selector is not valid:", selector, err);
                None
            }
        };
        let iframe = |selector: &str| {
            find(selector).and_then(|element| element.dyn_into::<HtmlIFrameElement>().ok())
        };

        let Some(now_frame) = iframe(".now .mirror") else {
            // Without a pane there is nothing to be a presenter view of, and
            // every method below would quietly do nothing. Say so once instead.
            error!("Presenter layout has no pane; the view will not show slides");
            return;
        };

        let status = find_status(&find);

        // `None` when the dialog is missing from the markup, which costs the
        // view its picker and nothing else — every call below is an `if let`.
        let picker = match (
            find(".picker").and_then(|element| element.dyn_into::<HtmlDialogElement>().ok()),
            find(".strip-grid"),
            find(".strip-search").and_then(|element| element.dyn_into::<HtmlInputElement>().ok()),
            find(".strip-count"),
            find(".strip-status"),
        ) {
            (Some(dialog), Some(grid), Some(input), Some(count), Some(status)) => {
                let mut picker = Picker::new(dialog, grid, input, count, status);
                picker.set_toggle(find(".strip-toggle"));
                Some(Rc::new(RefCell::new(picker)))
            }
            _ => None,
        };

        let inner = Rc::new(RefCell::new(Inner {
            layout: layout.clone(),
            now: Stage {
                frame: now_frame,
                last: None,
                ready: false,
            },
            notes: find(".notes"),
            next_shot: find(".next-shot"),
            next_title: find(".next-title"),
            next_number: find(".next-number"),
            status,
            picker: picker.clone(),
            thumbs: Thumbnails::default(),
            head: None,
            footer: None,
            lang: None,
            plan: Vec::new(),
            step_counts: Vec::new(),
            titles: Vec::new(),
            total_slides: 0,
            state_class: "init".to_owned(),
            current_index: 0,
            current_step: 0,
            current_slide: None,
            on_slide: false,
            elapsed: Elapsed::default(),
        }));

        self.install_inbox(&inner);
        self.install_buttons(&inner, &find);
        self.install_picker(&inner, picker.as_ref(), &find);
        self.install_notes_size(host, &find);
        self.install_stage_scale(&layout);

        // The timer is the one thing here that moves without the deck moving,
        // so it needs a clock of its own rather than a redraw per slide change.
        let ticking = Rc::clone(&inner);
        self.ticker = Some(Interval::new(TICK_MS, move || {
            ticking.borrow().refresh_status();
        }));

        inner.borrow().refresh_status();
        // Started before anything asks for a picture: the next pane wants one on
        // the first slide change, and the server is usually still photographing
        // the deck when this page opens.
        thumbnails::probe(Rc::clone(&inner), 0);
        self.inner = Some(inner);
    }
}

impl TobogganPresenterElement {
    /// Listens for the mirrors' handshakes.
    fn install_inbox(&mut self, inner: &Rc<RefCell<Inner>>) {
        let Some(origin) = mirror::page_origin() else {
            return;
        };
        let inner = Rc::clone(inner);
        self.inbox = Some(EventListener::new(&window(), "message", move |event| {
            let Some(event) = event.dyn_ref::<MessageEvent>() else {
                return;
            };
            match mirror::decode(event, &origin) {
                Some(MirrorMessage::Ready { pane }) => inner.borrow_mut().welcome(pane),
                // Frames only travel the other way; this view is never sent one.
                Some(MirrorMessage::Frame(_)) | None => (),
            }
        }));
    }

    /// Wires the navigation and the timer controls.
    fn install_buttons(
        &mut self,
        inner: &Rc<RefCell<Inner>>,
        find: &impl Fn(&str) -> Option<Element>,
    ) {
        // `NextStep`/`PreviousStep` rather than the slide commands, for the
        // reason already written in the keymap: a step command walks onto the
        // neighbouring slide once a slide's reveals run out, so two buttons
        // reach the whole deck — where `NextSlide` would leave every reveal
        // unreachable from them.
        if let Some(commands) = self.commands.clone() {
            for (selector, command) in [
                (".go-prev", Command::PreviousStep),
                (".go-next", Command::NextStep),
            ] {
                let Some(button) = find(selector) else {
                    continue;
                };
                let commands = commands.clone();
                self.listeners
                    .push(EventListener::new(&button, "click", move |_| {
                        if commands.unbounded_send(command.clone()).is_err() {
                            error!("The presenter view could not send a command");
                        }
                    }));
            }
        }

        for (selector, restart) in [(".pause", false), (".reset", true)] {
            let Some(button) = find(selector) else {
                continue;
            };
            let inner = Rc::clone(inner);
            self.listeners
                .push(EventListener::new(&button, "click", move |_| {
                    let mut inner = inner.borrow_mut();
                    if restart {
                        inner.elapsed.restart();
                    } else {
                        inner.elapsed.toggle();
                    }
                    inner.refresh_status();
                }));
        }
    }

    /// Wires the slide picker: its buttons, its search box, and the three keys
    /// that open it.
    ///
    /// The keys are caught here rather than added to [`crate::KeyboardMapping`],
    /// because the picker is a surface only this page has: a binding in the
    /// shared keymap would appear in the deck's own help dialog naming something
    /// the deck cannot do. The toggle button's tooltip names all three instead —
    /// the dialog's hint bar cannot, since it is only readable once the picker
    /// is already open.
    fn install_picker(
        &mut self,
        inner: &Rc<RefCell<Inner>>,
        picker: Option<&Rc<RefCell<Picker>>>,
        find: &impl Fn(&str) -> Option<Element>,
    ) {
        let Some(picker) = picker else {
            return;
        };

        for (selector, opens) in [(".strip-toggle", true), (".strip-close", false)] {
            let Some(button) = find(selector) else {
                continue;
            };
            let inner = Rc::clone(inner);
            let picker = Rc::clone(picker);
            self.listeners
                .push(EventListener::new(&button, "click", move |_| {
                    let (readiness, version) = {
                        let inner = inner.borrow();
                        (inner.thumbs.readiness, inner.thumbs.version)
                    };
                    let mut picker = picker.borrow_mut();
                    if opens {
                        picker.toggle(readiness, version);
                    } else {
                        picker.close();
                    }
                }));
        }

        if let Some(commands) = self.commands.clone() {
            self.listeners.push(picker::install_jump(picker, commands));
        }
        self.listeners.push(picker::install_close(picker));
        self.listeners.push(picker::install_search(picker));
        self.listeners.push(picker::install_shot_errors(picker));
        self.listeners
            .push(picker::install_navigation(picker, self.commands.clone()));

        // Bare `g` or `/`, and `Ctrl`/`Cmd`+`K` — the three a speaker reaches
        // for. `g` and `/` are both unbound in the deck's keymap, and a modified
        // key never reaches it at all, so none of the three is taken from the
        // deck.
        //
        // Only when nothing is being typed into, the same guard the deck's own
        // keymap applies for the same reason: a `g` at the quake terminal's
        // prompt is a letter, not a command. It is also what makes the picker's
        // own search box safe — `typing_into_editable` reads the event's
        // `composedPath`, so it sees an input inside this shadow root.
        let inner = Rc::clone(inner);
        let picker = Rc::clone(picker);
        let options = gloo::events::EventListenerOptions::enable_prevent_default();
        self.listeners.push(EventListener::new_with_options(
            &window(),
            "keydown",
            options,
            move |event| {
                let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                    return;
                };
                let modified = event.ctrl_key() || event.meta_key();
                let opens = match event.key().as_str() {
                    "g" | "/" => !modified && !event.alt_key(),
                    "k" => modified && !event.alt_key(),
                    _ => false,
                };
                if !opens || crate::deck_keys_captured() || crate::typing_into_editable(event) {
                    return;
                }
                // `Ctrl+K` and `/` are the browser's own before they are ours.
                event.prevent_default();
                let (readiness, version) = {
                    let inner = inner.borrow();
                    (inner.thumbs.readiness, inner.thumbs.version)
                };
                picker.borrow_mut().toggle(readiness, version);
            },
        ));
    }

    /// Wires `A−`/`A+`, and restores whatever size was last chosen.
    ///
    /// The notes are the surface a speaker actually reads mid-sentence, and how
    /// big they need to be is a fact about the room and the speaker's eyes, not
    /// about the deck — so it is remembered per browser rather than styled once.
    fn install_notes_size(&mut self, host: &HtmlElement, find: &impl Fn(&str) -> Option<Element>) {
        let size = Rc::new(RefCell::new(stored_notes_size()));
        apply_notes_size(host, *size.borrow());

        for (selector, delta) in [
            (".notes-smaller", -NOTES_SIZE_STEP),
            (".notes-bigger", NOTES_SIZE_STEP),
        ] {
            let Some(button) = find(selector) else {
                continue;
            };
            let host = host.clone();
            let size = Rc::clone(&size);
            self.listeners
                .push(EventListener::new(&button, "click", move |_| {
                    let mut size = size.borrow_mut();
                    *size = (*size + delta).clamp(NOTES_SIZE_RANGE.0, NOTES_SIZE_RANGE.1);
                    apply_notes_size(&host, *size);
                    store_notes_size(*size);
                }));
        }
    }

    /// Keeps `--stage-scale` in step with the size of the mirrored pane.
    ///
    /// The scale is a `transform`, which is paint-time and cannot change the
    /// size of the element being observed, so writing the property from inside
    /// the callback cannot start an observation loop.
    fn install_stage_scale(&mut self, layout: &Element) {
        let Ok(panes) = layout.query_selector_all(".now .pane.screen") else {
            return;
        };
        let panes = (0..panes.length())
            .filter_map(|index| panes.item(index))
            .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
            .collect::<Vec<_>>();

        let watched = panes.clone();
        let callback = Closure::<dyn FnMut()>::new(move || rescale_stages(&watched));
        // Once directly as well: `observe` does deliver an initial observation,
        // but it arrives in a later task, and until it does `var(--stage-scale,
        // 1)` is a 1280px iframe in a 400px box.
        rescale_stages(&panes);

        match ResizeObserver::new(callback.as_ref().unchecked_ref()) {
            Ok(observer) => {
                for pane in &panes {
                    observer.observe(pane);
                }
                self.scale = Some(observer);
                self.scale_callback = Some(callback);
            }
            Err(err) => error!("Presenter panes will not follow the window:", err),
        }
    }
}

/// Writes each pane's share of [`STAGE_WIDTH`] onto it.
fn rescale_stages(panes: &[HtmlElement]) {
    for pane in panes {
        let width = f64::from(pane.client_width());
        if width <= 0.0 {
            continue;
        }
        let _ = pane
            .style()
            .set_property("--stage-scale", &format!("{:.5}", width / STAGE_WIDTH));
    }
}

fn apply_notes_size(host: &HtmlElement, size: f64) {
    let _ = host
        .style()
        .set_property("--notes-size", &format!("{size}px"));
}

/// The notes size last chosen here, or the default.
///
/// Every failure is the default: a private window, storage turned off, or a
/// value edited by hand into something that is not a number.
fn stored_notes_size() -> f64 {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(NOTES_SIZE_KEY).ok().flatten())
        .and_then(|raw| raw.parse::<f64>().ok())
        .filter(|size| (NOTES_SIZE_RANGE.0..=NOTES_SIZE_RANGE.1).contains(size))
        .unwrap_or(NOTES_SIZE_DEFAULT)
}

fn store_notes_size(size: f64) {
    if let Ok(Some(storage)) = window().local_storage() {
        let _ = storage.set_item(NOTES_SIZE_KEY, &size.to_string());
    }
}
