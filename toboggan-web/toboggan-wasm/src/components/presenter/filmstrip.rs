//! The whole deck at a glance, and a way to jump into it.
//!
//! The presenter view shows two slides: the one the room is looking at and the
//! one after it. Everything else the speaker has to remember, or reach by typing
//! a slide number blind. This is the third view — every slide at once — and it
//! is the one thing the slide-overview thumbnails are genuinely better at than a
//! live mirror: forty small stills is forty pictures, where forty iframes is
//! forty copies of the deck.
//!
//! Two things it must get right, and neither is obvious:
//!
//! * **One index space.** A cell's position is a *presented* slide index — what
//!   [`Command::GoTo`] takes — while the thumbnails on disk are named over the
//!   deck as authored, which includes the `hidden_in = ["web"]` slides the room
//!   never sees. The server crosses between them behind `/overview/slide/{index}`
//!   so this file never has to know the deck hides anything.
//! * **Thumbnails may not exist yet.** They are generated on first request and a
//!   deck takes seconds. An `<img>` pointed at a thumbnail that is not ready
//!   gets a `503` and stays broken for ever, so the strip asks *once*, over
//!   `fetch`, where it can tell "still working" (retry) from "never going to
//!   work" (say so and stop).

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::error;
use gloo::events::EventListener;
use gloo::net::http::Request;
use gloo::timers::callback::Timeout;
use gloo::utils::window;
use toboggan_core::{Command, SlideId};
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::spawn_local;
use web_sys::Element;

use crate::create_html_element;
use crate::utils::errors::log_dom_error;

/// How long to wait before asking again whether the thumbnails are ready.
///
/// The same order as the generation itself — a few seconds for a deck — so a
/// speaker who opens the strip on a cold server sees it fill in rather than
/// watching a spinner that polls twenty times a second.
const RETRY_MS: u32 = 1_000;

/// Whether the thumbnails behind the strip can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readiness {
    /// Not asked yet, or asking again.
    Unknown,
    /// The server has thumbnails; the cells carry real `src`s.
    Ready,
    /// The server cannot make them — no browser and no `typst`, or a deck that
    /// failed to render. Said once, and not retried: a strip that polls for ever
    /// is a strip that is lying about being about to work.
    Unavailable,
}

/// The slide grid: its DOM, and what it currently believes.
pub(super) struct Filmstrip {
    panel: Element,
    grid: Element,
    status: Element,
    toggle: Option<Element>,
    /// One button per presented slide, in order.
    cells: Vec<Element>,
    /// Bumped on every reload, and appended to each `src`.
    ///
    /// The thumbnail of slide 4 is a *different picture* at the same URL after
    /// the deck reloads, and the route says `no-cache` — but a cache is not the
    /// only thing that would hold the old one: an `<img>` whose `src` is set to
    /// the string it already has does not re-fetch at all.
    version: u32,
    open: bool,
    readiness: Readiness,
    current: usize,
    /// Held so it stays armed; dropped to stop asking.
    retry: Option<Timeout>,
}

impl Filmstrip {
    /// Wires the panel that [`super::layout_html`] wrote.
    pub(super) fn new(panel: Element, grid: Element, status: Element) -> Self {
        Self {
            panel,
            grid,
            status,
            toggle: None,
            cells: Vec::new(),
            version: 0,
            open: false,
            readiness: Readiness::Unknown,
            current: 0,
            retry: None,
        }
    }

    /// The button that opens the strip, so its `aria-expanded` can be kept true.
    pub(super) fn set_toggle(&mut self, toggle: Option<Element>) {
        self.toggle = toggle;
    }

    pub(super) const fn is_open(&self) -> bool {
        self.open
    }

    /// Marks which cell the deck is on, and brings it into view.
    ///
    /// Scrolled only while the strip is open: scrolling a hidden element moves
    /// it to a position the speaker never asked for, and the next open would
    /// start halfway down the deck.
    pub(super) fn set_current(&mut self, current: usize) {
        self.current = current;
        for (index, cell) in self.cells.iter().enumerate() {
            if index == current {
                let _ = cell.set_attribute("aria-current", "true");
                if self.open {
                    cell.scroll_into_view();
                }
            } else {
                let _ = cell.remove_attribute("aria-current");
            }
        }
    }

    /// The deck changed under the speaker: every picture is now potentially a
    /// different picture, and there may be a different number of them.
    ///
    /// Takes the handle as well as `&mut self` because a reload that lands while
    /// the strip is *open* has to start asking again — the thumbnails are being
    /// regenerated and the answer will change. Without that the strip sat on
    /// "Rendering slide previews…" until it was closed and reopened, since
    /// nothing but [`Self::show`] ever started a probe.
    pub(super) fn invalidate(&mut self, handle: &Rc<RefCell<Self>>, total: usize) {
        self.version = self.version.wrapping_add(1);
        // A failed deck may render on the next save, so a reload is also the one
        // event that earns a retry after [`Readiness::Unavailable`].
        self.readiness = Readiness::Unknown;
        // Dropping the old timer cancels it, so the previous version's chain
        // does not go on asking beside the new one.
        self.retry = None;
        self.build(total);
        if self.open {
            self.refresh();
            probe(Rc::clone(handle), self.version);
        }
    }

    /// Builds or trims the grid so there is exactly one cell per slide.
    pub(super) fn build(&mut self, total: usize) {
        while self.cells.len() > total {
            if let Some(cell) = self.cells.pop() {
                let _ = self.grid.remove_child(&cell);
            }
        }
        while self.cells.len() < total {
            let index = self.cells.len();
            let Some(cell) = self.make_cell(index) else {
                return;
            };
            self.cells.push(cell);
        }
    }

    /// One cell, carrying its own slide index for [`install_jump`] to read back.
    ///
    /// The index is written into the DOM rather than captured in a closure per
    /// cell, so a forty-slide deck installs one listener instead of forty — and
    /// so rebuilding the grid on a live reload cannot leave the old ones behind.
    fn make_cell(&mut self, index: usize) -> Option<Element> {
        let cell = create_html_element("button");
        cell.set_class_name("strip-cell");
        let _ = cell.set_attribute("type", "button");
        let _ = cell.set_attribute("data-slide", &index.to_string());
        // 1-based, the way every other number the speaker reads is.
        let number = index + 1;
        let _ = cell.set_attribute("aria-label", &format!("Go to slide {number}"));
        cell.set_inner_html(&format!(
            r#"<img alt="" loading="lazy"><span class="strip-number">{number}</span>"#
        ));

        if let Err(err) = self.grid.append_child(&cell) {
            log_dom_error("append a filmstrip cell", &err);
            return None;
        }
        Some(cell.unchecked_into())
    }

    /// Shows the strip if it is hidden, hides it if it is not.
    pub(super) fn toggle(&mut self, inner: &Rc<RefCell<Self>>) {
        if self.open {
            self.close();
        } else {
            self.show(inner);
        }
    }

    pub(super) fn show(&mut self, inner: &Rc<RefCell<Self>>) {
        self.open = true;
        let _ = self.panel.remove_attribute("hidden");
        if let Some(toggle) = &self.toggle {
            let _ = toggle.set_attribute("aria-expanded", "true");
        }
        self.refresh();
        if self.readiness == Readiness::Unknown {
            // The version is read here and passed down rather than looked up
            // inside `probe`: this method holds the only `RefMut` there is, and
            // a synchronous `borrow()` on the way in panics on it.
            probe(Rc::clone(inner), self.version);
        }
        // After `refresh`, which is what put the pictures in the cells.
        self.set_current(self.current);
    }

    pub(super) fn close(&mut self) {
        self.open = false;
        let _ = self.panel.set_attribute("hidden", "");
        if let Some(toggle) = &self.toggle {
            let _ = toggle.set_attribute("aria-expanded", "false");
        }
        // Stops the polling with the panel: nobody is looking.
        self.retry = None;
    }

    /// Points every cell at its thumbnail, or says why it cannot.
    fn refresh(&mut self) {
        match self.readiness {
            Readiness::Ready => {
                self.status.set_text_content(None);
                let version = self.version;
                for (index, cell) in self.cells.iter().enumerate() {
                    let Ok(Some(image)) = cell.query_selector("img") else {
                        continue;
                    };
                    let _ = image.set_attribute("src", &thumbnail_src(index, version));
                }
            }
            Readiness::Unknown => self
                .status
                .set_text_content(Some("Rendering slide previews…")),
            Readiness::Unavailable => self
                .status
                .set_text_content(Some("Slide previews are unavailable on this machine.")),
        }
        let state = match self.readiness {
            Readiness::Ready => "ready",
            Readiness::Unknown => "pending",
            Readiness::Unavailable => "unavailable",
        };
        let _ = self.panel.set_attribute("data-previews", state);
    }
}

/// Where one presented slide's picture lives.
///
/// A presented index, not an authored one — see this module's header. The
/// version is a cache-buster; the route ignores it.
fn thumbnail_src(index: usize, version: u32) -> String {
    format!("/overview/slide/{index}?v={version}")
}

/// Asks the server whether the thumbnails are ready, and keeps asking while they
/// are merely late.
///
/// `fetch` rather than an `<img>`'s `error` event, because the two answers that
/// matter are indistinguishable to an image: `503` means the deck is still being
/// photographed and this is worth repeating, `404` means it never will be and
/// repeating it is a lie told once a second for the length of the talk.
fn probe(strip: Rc<RefCell<Filmstrip>>, version: u32) {
    // Slide 0 stands for all of them: they are made in one pass.
    //
    // `version` is a parameter rather than read from `strip`, because every
    // caller is either holding a `RefMut` on it or is a timer armed by one.
    // Nothing here may touch the cell until the `await` below has yielded.
    let url = thumbnail_src(0, version);
    spawn_local(async move {
        let readiness = match Request::get(&url).send().await {
            Ok(response) if response.ok() => Readiness::Ready,
            Ok(response) if response.status() == 503 => Readiness::Unknown,
            // A `404` is the server saying it cannot make them, and a transport
            // error is a server that is not there — neither improves by asking
            // again on a timer.
            Ok(_) | Err(_) => Readiness::Unavailable,
        };

        let mut inner = strip.borrow_mut();
        // The speaker closed the panel while the request was in flight.
        if !inner.open {
            return;
        }
        // A reload landed while the request was in flight, and this answer is
        // about the previous set of pictures — which were ready, while the ones
        // now being made are not. Believing it would put broken images in the
        // grid and stop anything asking again.
        if inner.version != version {
            return;
        }
        inner.readiness = readiness;
        inner.refresh();
        if readiness == Readiness::Ready {
            inner.retry = None;
            let current = inner.current;
            inner.set_current(current);
            return;
        }
        if readiness == Readiness::Unknown {
            let again = Rc::clone(&strip);
            inner.retry = Some(Timeout::new(RETRY_MS, move || probe(again, version)));
        }
    });
}

/// Sends the jump a click on a cell means, and closes the strip behind it.
///
/// One delegated listener on the grid rather than one per cell: the cells are
/// rebuilt whenever the deck reloads, and per-cell closures would have to be
/// dropped in step with them or go on answering for elements no longer in the
/// document.
///
/// Closing is the point. The strip is a full-height overlay, so leaving it up
/// after a jump hides the very slide the speaker jumped to — along with the
/// notes they jumped to it to read.
pub(super) fn install_jump(
    strip: &Rc<RefCell<Filmstrip>>,
    commands: UnboundedSender<Command>,
) -> EventListener {
    let grid = strip.borrow().grid.clone();
    let strip = Rc::clone(strip);
    EventListener::new(&grid, "click", move |event| {
        let Some(index) = event
            .target()
            .and_then(|target| target.dyn_into::<Element>().ok())
            .and_then(|element| element.closest(".strip-cell").ok().flatten())
            .and_then(|cell| cell.get_attribute("data-slide"))
            .and_then(|value| value.parse::<usize>().ok())
        else {
            // A click on the gap between cells.
            return;
        };

        let command = Command::GoTo {
            slide: SlideId::new(index),
        };
        if commands.unbounded_send(command).is_err() {
            error!("The filmstrip could not send a jump");
        }
        strip.borrow_mut().close();
    })
}

/// Closes the strip on `Escape`, wherever the focus happens to be.
///
/// On `window` rather than the panel: the speaker opened this with a keystroke
/// and nothing in it is focused, so a listener on the panel would never hear
/// anything. The deck's own keymap does not bind `Escape`, so nothing is taken
/// from it.
pub(super) fn install_escape(strip: &Rc<RefCell<Filmstrip>>) -> EventListener {
    let strip = Rc::clone(strip);
    EventListener::new(&window(), "keydown", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
            return;
        };
        if event.key() != "Escape" {
            return;
        }
        let mut strip = strip.borrow_mut();
        if strip.is_open() {
            strip.close();
        }
    })
}
