//! The presenter view: what the speaker looks at while the room looks at the
//! deck.
//!
//! Everything here is a *second* rendering of state the deck already has. The
//! view sends no commands of its own beyond the keyboard the deck already
//! provides, so a presenter can drive from it, mirror it, or close it, and the
//! talk is unaffected.

use std::cell::RefCell;
use std::rc::Rc;

use gloo::console::error;
use gloo::events::EventListener;
use gloo::timers::callback::Interval;
use toboggan_core::{Content, Slide, SlideId, State};
use wasm_bindgen::JsCast as _;
use web_sys::{Element, HtmlElement};

use crate::components::{TobogganSlideElement, WasmElement};
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try, render_content};

const CSS: &str = include_str!("style.css");

/// How often the clock and the elapsed timer redraw.
const TICK_MS: u32 = 1_000;

/// The bits of the status strip that change.
struct StatusBar {
    clock: Element,
    elapsed: Element,
    progress: Element,
    counter: Element,
    steps: Element,
    pacing: Element,
}

/// The parts the ticker and the state updates both touch.
struct Inner {
    next: TobogganSlideElement,
    current_host: Option<HtmlElement>,
    notes: Option<Element>,
    status: Option<StatusBar>,
    /// Planned seconds per slide, from the deck's `duration` front matter.
    /// Empty when the deck plans nothing, which hides the pacing readout.
    plan: Vec<Option<u64>>,
    /// Reveal steps per slide, so the counter can say `2/3` rather than `2`.
    step_counts: Vec<usize>,
    total_slides: usize,
    /// Wall-clock milliseconds when the talk started, by this view's reckoning.
    ///
    /// Set the first time the deck is seen running, so opening the view before
    /// the talk does not have it counting the coffee break — and resettable,
    /// because the other case is opening it *after* the talk started.
    started_at: Option<f64>,
    current_index: usize,
}

#[derive(Default)]
pub(crate) struct TobogganPresenterElement {
    inner: Option<Rc<RefCell<Inner>>>,
    /// Held only to keep them alive for the page's lifetime — dropping the
    /// interval stops the clock, dropping the listener stops the reset.
    ticker: Option<Interval>,
    reset: Option<EventListener>,
}

impl TobogganPresenterElement {
    /// The element the *current* slide should be rendered into.
    ///
    /// Handed back rather than rendered here so the presenter view shows the
    /// same slide component the deck does, fed by the same code path — one
    /// renderer, two places to look at it.
    pub(crate) fn current_slide_host(&self) -> Option<HtmlElement> {
        self.inner.as_ref()?.borrow().current_host.clone()
    }

    /// Records the deck's shape: how many slides there are, how many reveals
    /// each has, and how long each is meant to take.
    pub(crate) fn set_plan(
        &self,
        total_slides: usize,
        plan: Vec<Option<u64>>,
        step_counts: Vec<usize>,
    ) {
        let Some(inner) = &self.inner else {
            return;
        };
        let mut inner = inner.borrow_mut();
        inner.total_slides = total_slides;
        inner.plan = plan;
        inner.step_counts = step_counts;
        inner.refresh_status();
    }

    /// Shows the slide after the current one, or clears the pane at the end.
    pub(crate) fn set_next(&mut self, slide: Option<Slide>) {
        let Some(inner) = &self.inner else {
            return;
        };
        // Every reveal shown at once: the point of the pane is to see what is
        // coming, not to re-enact its build.
        inner.borrow_mut().next.set_slide(slide, usize::MAX);
    }

    /// Takes the deck's current position and redraws everything derived from it.
    pub(crate) fn set_state(&self, state: &State, notes: Option<&Content>) {
        let Some(inner) = &self.inner else {
            return;
        };
        let mut inner = inner.borrow_mut();

        if inner.started_at.is_none() && state.current().is_some() {
            inner.started_at = Some(js_sys::Date::now());
        }
        inner.current_index = state.current().map_or(0, SlideId::index);

        if let Some(notes_element) = &inner.notes {
            let html = notes.map(|notes| render_content(notes, None));
            notes_element.set_inner_html(html.as_deref().unwrap_or_default());
        }

        inner.refresh_steps(state);
        inner.refresh_status();
    }
}

impl Inner {
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

        let elapsed = self.elapsed_secs();
        status
            .elapsed
            .set_text_content(Some(&format!("⏱ {}", format_duration(elapsed))));

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

    fn elapsed_secs(&self) -> u64 {
        let Some(started_at) = self.started_at else {
            return 0;
        };
        let millis = (js_sys::Date::now() - started_at).max(0.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let secs = (millis / 1_000.0) as u64;
        secs
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

impl WasmElement for TobogganPresenterElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create presenter shadow root"
        );

        let layout: Element = dom_try!(create_and_append_element(&root, "div"), "presenter layout");
        layout.set_class_name("layout");
        layout.set_inner_html(
            r#"<div class="now"><div class="pane screen"><div class="fit"></div></div></div>
<div class="aside">
  <p class="label">Next</p>
  <div class="next"><div class="pane screen"><div class="fit"></div></div></div>
  <div class="pane notes"></div>
</div>
<div class="status">
  <span class="clock"></span>
  <button type="button" class="elapsed" title="Click to restart the timer"></button>
  <span class="progress"><span></span></span>
  <span class="counter"></span>
  <span class="steps"></span>
  <span class="pacing"></span>
</div>"#,
        );

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
        // The slide component attaches its shadow root to whatever host it is
        // handed, so the host has to be an element we are willing to give away
        // entirely — hence `.fit` inside the pane rather than the pane itself.
        // Scaling the pane would scale its frame and its aspect ratio with it.
        let current_host = find(".now .fit").and_then(|el| el.dyn_into::<HtmlElement>().ok());
        let next_host = find(".next .fit").and_then(|el| el.dyn_into::<HtmlElement>().ok());

        let mut next = TobogganSlideElement::default();
        next.set_preview(true);
        match &next_host {
            Some(next_host) => next.render(next_host),
            // Not silent: without this the "next slide" pane simply never
            // renders, which reads as "there is no next slide".
            None => error!("Presenter layout has no pane for the next slide"),
        }

        let status = match (
            find(".clock"),
            find(".elapsed"),
            find(".progress"),
            find(".counter"),
            find(".steps"),
            find(".pacing"),
        ) {
            (
                Some(clock),
                Some(elapsed),
                Some(progress),
                Some(counter),
                Some(steps),
                Some(pacing),
            ) => Some(StatusBar {
                clock,
                elapsed,
                progress,
                counter,
                steps,
                pacing,
            }),
            _ => None,
        };

        let inner = Rc::new(RefCell::new(Inner {
            next,
            current_host,
            notes: find(".notes"),
            status,
            plan: Vec::new(),
            step_counts: Vec::new(),
            total_slides: 0,
            started_at: None,
            current_index: 0,
        }));

        // The timer is the one thing here that moves without the deck moving,
        // so it needs a clock of its own rather than a redraw per slide change.
        let ticking = Rc::clone(&inner);
        self.ticker = Some(Interval::new(TICK_MS, move || {
            ticking.borrow().refresh_status();
        }));

        // Opening the view mid-talk starts the timer from zero, which is wrong
        // in the other direction; clicking it says "the talk started now".
        if let Some(button) = inner
            .borrow()
            .status
            .as_ref()
            .map(|status| status.elapsed.clone())
        {
            let resetting = Rc::clone(&inner);
            self.reset = Some(EventListener::new(&button, "click", move |_| {
                let mut inner = resetting.borrow_mut();
                inner.started_at = Some(js_sys::Date::now());
                inner.refresh_status();
            }));
        }

        inner.borrow().refresh_status();
        self.inner = Some(inner);
    }
}
