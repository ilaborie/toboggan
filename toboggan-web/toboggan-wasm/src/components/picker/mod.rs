//! The whole deck at a glance, searchable, and a way to jump into it.
//!
//! A page showing a deck shows one slide of it. Everything else the speaker has
//! to remember, or reach by typing a slide number blind. This is the surface
//! that shows all of them at once, with a search box over it, and it is the one
//! thing the slide-overview thumbnails are genuinely better at than a live
//! mirror: forty small stills is forty pictures, where forty iframes is forty
//! copies of the deck.
//!
//! It is a component rather than part of the presenter view because nothing in
//! it is the speaker's chrome: it asks the server for the deck's words
//! (`GET /api/outline`) and the deck's photographs (`/overview/slide/{index}`),
//! and it writes back one [`Command::GoTo`]. A host gives it somewhere to mount
//! and a channel to jump on.
//!
//! Four things it must get right, and none is obvious:
//!
//! * **One index space.** A cell's position is a *presented* slide index — what
//!   [`Command::GoTo`] takes — while the thumbnails on disk are named over the
//!   deck as authored, which includes the `hidden_in = ["web"]` slides the room
//!   never sees. The server crosses between them behind `/overview/slide/{index}`
//!   so this file never has to know the deck hides anything.
//! * **An open picker owns the keyboard.** While the dialog is up it holds a
//!   [`claim_keys_for_modal`] guard, so the deck's whole keymap stands down and
//!   the arrows, the digits and `Enter` are the picker's whatever inside it has
//!   focus. Focus alone is not enough: [`crate::typing_into_editable`] speaks
//!   for a text field, so a click on the hint bar used to move focus off the
//!   search box and leave `ArrowRight` advancing the room's slide behind the
//!   open dialog — and `Enter` racing a jump against a digit the deck had also
//!   heard. The claim is a flag rather than `stopPropagation` because the
//!   search box sits on the path down from `window` too.
//! * **A `<dialog>`, not a panel.** `showModal` puts it in the top layer, traps
//!   the focus and closes on `Escape` without a listener — and the `close` event
//!   it fires is the one place the open state has to be kept in step, however
//!   the speaker got out of it.
//! * **A shadow root of its own.** The top layer is not a style boundary: a
//!   dialog opened over the deck is still in the deck's document, where
//!   `_head.html` is arbitrary author CSS. Mounted in the presenter view the
//!   nesting costs nothing; mounted on `/run` it is the only thing standing
//!   between this grid and a stylesheet written for slides.

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc::UnboundedSender;
use gloo::console::error;
use gloo::events::EventListener;
use gloo::utils::{document, window};
use toboggan_core::{Command, SlideId, SlideOutline};
use wasm_bindgen::JsCast as _;
use web_sys::{Element, HtmlDialogElement, HtmlElement, HtmlInputElement};

use crate::components::WasmElement;
use crate::components::thumbnails::{self, Readiness, Redraw, Thumbnails, thumbnail_src};
use crate::utils::errors::log_dom_error;
use crate::{
    ModalKeys, ToastType, claim_keys_for_modal, create_and_append_element, create_html_element,
    create_shadow_root_with_style, dom_try, notify,
};

const CSS: &str = include_str!("style.css");

/// What the dialog contains.
///
/// Out of line from [`WasmElement::render`] only because it is markup: every
/// selector that page queries is matched against this.
const MARKUP: &str = r#"<div class="strip-head">
  <input class="strip-search" type="search" placeholder="Find a slide…" aria-label="Find a slide"
         autocomplete="off" autofocus role="combobox" aria-expanded="true" aria-controls="slide-grid">
  <span class="strip-count"></span>
  <span class="strip-status"></span>
  <button type="button" class="strip-close" title="Close (Esc)" aria-label="Close the slide picker">✕</button>
</div>
<div class="strip-grid" id="slide-grid" role="listbox" aria-label="Slides"></div>
<p class="strip-hint">type to filter · ↑↓←→ move · ⏎ go · Esc close</p>"#;

/// What the hint bar says on a client that may not move the deck.
///
/// Standing text rather than a message on the first refused click: an audience
/// member should be able to see that the grid is to read and not to drive
/// *before* they press anything.
const AUDIENCE_HINT: &str = "type to filter · ↑↓←→ move · Esc close — watching, not presenting";

/// The slide picker, as a page mounts it.
///
/// The state behind it is shared with the listeners that drive it, so every
/// method here takes `&self`: what a host holds is a handle, not the picker.
#[derive(Default)]
pub(crate) struct TobogganPickerElement {
    /// Where a jump is sent. Set before [`WasmElement::render`], which is where
    /// the cells and the keys are wired. Without it the picker still opens,
    /// searches and closes — it is a way of *reading* the deck as much as a way
    /// of moving it — and only the jump is gone.
    commands: Option<UnboundedSender<Command>>,
    /// Shared with whatever else on the page is made of photographs, which is
    /// the presenter view's next-slide pane: one probe then answers for both,
    /// and a reload invalidates one set of pictures rather than two.
    thumbs: Rc<RefCell<Thumbnails>>,
    /// `None` until `render`, and after a `render` that could not find its own
    /// markup. Every method below is written to do nothing in that case rather
    /// than to half-work.
    picker: Option<Rc<RefCell<Picker>>>,
    /// Held only to keep them alive for the page's lifetime — dropping a
    /// listener stops its button, its search box, or the keys that open the
    /// dialog.
    listeners: Vec<EventListener>,
    /// Asks the server about the photographs again, if anything is worth
    /// asking. Built in [`WasmElement::render`], because it redraws the picker
    /// that call creates.
    ///
    /// Run whenever the picker is opened: a probe that ran out of patience with
    /// a deck the server was still photographing leaves nothing asking, and the
    /// speaker opening the grid is exactly the moment the answer is wanted.
    resume: Option<Rc<dyn Fn()>>,
}

impl TobogganPickerElement {
    /// Points a jump at the channel the host's own navigation writes to. Must be
    /// called before [`WasmElement::render`].
    ///
    /// The same channel, deliberately: that is the one place a client which may
    /// not present is refused and told why, and a second path would be a second
    /// copy of the rule that can disagree with it.
    pub(crate) fn set_commands(&mut self, commands: UnboundedSender<Command>) {
        self.commands = Some(commands);
    }

    /// Shares the host's photographs, so a host that draws its own draws them
    /// from the same verdict and the same generation. Must be called before
    /// [`WasmElement::render`].
    ///
    /// A host with no other pictures leaves this alone and the picker probes
    /// against one of its own.
    pub(crate) fn set_thumbnails(&mut self, thumbs: Rc<RefCell<Thumbnails>>) {
        self.thumbs = thumbs;
    }

    /// The host's button that opens the picker, if it has one, so its
    /// `aria-expanded` can be kept in step with the dialog.
    ///
    /// A button with no dialog behind it is hidden rather than kept: [`Self::with`]
    /// would swallow the click, and a control that is drawn, tooltipped and dead
    /// tells the speaker mid-talk that the picker is broken in a way they cannot
    /// act on. Nothing to open is better said by nothing to press.
    pub(crate) fn set_toggle(&self, toggle: Option<Element>) {
        if self.picker.is_none() {
            if let Some(toggle) = &toggle {
                let _ = toggle.set_attribute("hidden", "");
            }
            return;
        }
        self.with(|picker| picker.set_toggle(toggle));
    }

    /// Takes the deck's searchable text — see [`Picker::set_outline`].
    pub(crate) fn set_outline(&self, slides: &[SlideOutline]) {
        self.with(|picker| picker.set_outline(slides));
    }

    /// Drops the corpus, because the deck it described is gone.
    pub(crate) fn forget_outline(&self) {
        self.with(Picker::forget_outline);
    }

    /// Reports that the deck's words could not be fetched — see
    /// [`Picker::note_outline_failed`].
    pub(crate) fn note_outline_failed(&self) {
        self.with(Picker::note_outline_failed);
    }

    /// Builds one cell per slide, for a deck whose length is known before its
    /// words are.
    pub(crate) fn build(&self, total: usize) {
        self.with(|picker| picker.build(total));
    }

    /// Marks the cell the deck is on.
    pub(crate) fn set_current(&self, current: usize) {
        self.with(|picker| picker.set_current(current));
    }

    /// Whether this client may move the deck — see [`Picker::set_can_drive`].
    pub(crate) fn set_can_drive(&self, can_drive: bool) {
        self.with(|picker| picker.set_can_drive(can_drive));
    }

    /// Repaints the grid from what the thumbnails now say.
    pub(crate) fn refresh(&self) {
        self.with(Picker::refresh);
    }

    /// The deck reloaded: the corpus and every picture describe a deck that is
    /// gone. Rebuilds the grid to the new length and starts a fresh probe.
    ///
    /// For a host whose only photographs are this picker's — the deck page. The
    /// presenter view drives the [`Thumbnails`] it shares instead, because a
    /// verdict there repaints its next-slide pane too, and two probes racing on
    /// one cell would each be answering the other's question.
    pub(crate) fn reload(&self, total: usize) {
        // The corpus first, and before the outline is asked for again: a picker
        // searching text one reload out of date answers a query with a slide
        // that no longer says what was searched for, and jumps there
        // confidently.
        self.with(|picker| {
            picker.forget_outline();
            picker.build(total);
        });
        let version = {
            let mut thumbs = self.thumbs.borrow_mut();
            thumbs.invalidate();
            thumbs.begin_probe()
        };
        self.probe(version);
    }

    /// The same deck, seen again: a first load, or a socket that dropped and
    /// came back.
    ///
    /// Sizes the grid, and asks about the photographs only if nothing is asking
    /// yet. Emphatically *not* [`Self::reload`]: a reconnect is not a reload,
    /// and treating it as one threw away the search corpus and every picture on
    /// each blip of a room's wifi — re-fetching the whole thumbnail set under a
    /// new version, and blanking the grid to "Rendering slide previews…" in
    /// front of a speaker who was reading it.
    pub(crate) fn sync(&self, total: usize) {
        self.with(|picker| picker.build(total));
        let version = self.thumbs.borrow_mut().begin_probe();
        self.probe(version);
    }

    /// Starts a probe chain for a claim [`Thumbnails::begin_probe`] handed out.
    fn probe(&self, version: Option<u32>) {
        if let Some(version) = version {
            thumbnails::probe(&self.thumbs, version, self.redraw());
        }
    }

    /// What a probe's verdict is worth to this picker: its grid, repainted.
    fn redraw(&self) -> Redraw {
        match &self.picker {
            Some(picker) => redraw_for(picker),
            None => Rc::new(|| ()),
        }
    }

    /// Opens the picker, or closes it if it is already open.
    pub(crate) fn toggle(&self) {
        if let Some(resume) = &self.resume {
            resume();
        }
        self.with(Picker::toggle);
    }

    /// Runs `action` against the picker, if there is one.
    ///
    /// One place for that `if`, rather than one per method: a picker whose
    /// markup did not come up is a picker the host goes on calling, and every
    /// one of those calls has the same nothing to do.
    ///
    /// Deliberately silent, because these run per keystroke and per probe — the
    /// one report is the error [`WasmElement::render`] logs when it gives up,
    /// which names the cause. What must not be silent is a *control* that
    /// pretends otherwise: see [`Self::set_toggle`].
    fn with(&self, action: impl FnOnce(&mut Picker)) {
        if let Some(picker) = &self.picker {
            action(&mut picker.borrow_mut());
        }
    }
}

impl WasmElement for TobogganPickerElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create the picker's shadow root"
        );
        let dialog = dom_try!(
            create_and_append_element::<HtmlDialogElement>(&root, "dialog"),
            "the picker's dialog"
        );
        dialog.set_class_name("picker");
        dialog.set_inner_html(MARKUP);

        // Each miss is named: every selector here is matched against `MARKUP`
        // sixty lines above, so it fires exactly when someone edits that.
        let find = |selector: &str| match dialog.query_selector(selector) {
            Ok(Some(element)) => Some(element),
            Ok(None) => {
                error!("The picker's markup is missing an element:", selector);
                None
            }
            Err(err) => {
                error!("The picker's selector is not valid:", selector, err);
                None
            }
        };
        let close_button = find(".strip-close");
        let (Some(grid), Some(input), Some(count), Some(status)) = (
            find(".strip-grid"),
            find(".strip-search").and_then(|element| element.dyn_into::<HtmlInputElement>().ok()),
            find(".strip-count"),
            find(".strip-status"),
        ) else {
            // A half-wired picker is worse than none: it would open on a grid
            // nothing fills, over a deck the speaker can no longer see.
            //
            // Said out loud, and once, here. From this point the host holds a
            // handle whose every method is a no-op (see [`Self::with`]) — so
            // without this line the only trace of the failure would be the
            // individual `find` misses above, logged at load and never
            // connected by anyone to a `g` that stops working minutes later.
            error!("The slide picker has no markup to drive; it will not open");
            return;
        };

        let picker = Rc::new(RefCell::new(Picker::new(
            dialog,
            grid,
            input,
            count,
            status,
            Rc::clone(&self.thumbs),
        )));

        self.listeners
            .push(install_jump(&picker, self.commands.clone()));
        self.listeners.push(install_close(&picker));
        self.listeners.push(install_search(&picker));
        self.listeners.push(install_shot_errors(&picker));
        self.listeners
            .push(install_navigation(&picker, self.commands.clone()));
        // Built from the local handle rather than from `self.redraw()`: this
        // runs before `self.picker` is set, and a redraw built from the field
        // now would capture the `None` it currently holds and repaint nothing
        // for the life of the page.
        let resume = {
            let thumbs = Rc::clone(&self.thumbs);
            let redraw = redraw_for(&picker);
            let resume: Rc<dyn Fn()> = Rc::new(move || {
                let version = thumbs.borrow_mut().begin_probe();
                if let Some(version) = version {
                    thumbnails::probe(&thumbs, version, Rc::clone(&redraw));
                }
            });
            resume
        };
        self.listeners
            .push(install_keys(&picker, Rc::clone(&resume)));
        self.resume = Some(resume);
        if let Some(button) = close_button {
            let handle = Rc::clone(&picker);
            self.listeners
                .push(EventListener::new(&button, "click", move |_| {
                    handle.borrow_mut().close();
                }));
        }

        self.picker = Some(picker);
    }
}

/// How much of a slide's text to show around the first hit.
const SNIPPET_CHARS: usize = 90;

/// One slide as the search sees it.
///
/// Everything folded is folded once, when the outline lands, rather than on
/// every keystroke: the deck does not change between two characters of a query,
/// and a speaker types this while a room waits. `haystack` answers "does this
/// slide match"; `folded_body` and `folded_notes` are what a snippet is located
/// in, and they are kept apart from `haystack` because a snippet has to be cut
/// out of one field rather than out of all of them joined.
struct Entry {
    title: String,
    body: String,
    notes: String,
    haystack: String,
    folded_body: String,
    folded_notes: String,
}

/// The slide picker: its DOM, its corpus, and what the query left of it.
/// Whether the deck's words are here to be searched.
///
/// Kept apart from `entries.is_empty()`, which cannot tell "not yet" and "never"
/// from a deck of no slides — and the difference is the whole of what the count
/// is allowed to claim. See [`Picker::apply_query`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Corpus {
    /// `GET /api/outline` has not answered yet.
    #[default]
    Pending,
    /// The deck's words are in `entries`.
    Loaded,
    /// The ask failed. The grid is still worth having — it still shows the deck
    /// and still jumps — but nothing can be filtered.
    Failed,
}

struct Picker {
    dialog: HtmlDialogElement,
    grid: Element,
    input: HtmlInputElement,
    count: Element,
    status: Element,
    toggle: Option<Element>,
    /// One cell per presented slide, in deck order.
    cells: Vec<Element>,
    /// The searchable deck, in cell order. Empty until `GET /api/outline`
    /// answers, and empty for good if it never does — `corpus` is what says
    /// which of the two an empty one is.
    entries: Vec<Entry>,
    /// Which slides the query left, as deck indices, in deck order. No scoring:
    /// a grid that reshuffles under the eye is a grid the speaker has to read
    /// again from the top.
    matches: Vec<usize>,
    /// Where the selection is, as a position in `matches`.
    selected: usize,
    /// Where the deck is, as a deck index.
    current: usize,
    /// Whether the deck's photographs exist yet, and which generation of them.
    /// Held rather than passed in per call: the picker repaints itself from a
    /// probe it did not start, and a host that has to remember to hand it a
    /// verdict is a host that can hand it a stale one.
    thumbs: Rc<RefCell<Thumbnails>>,
    /// Whether the query can be answered at all.
    corpus: Corpus,
    /// Whether this client may move the deck.
    ///
    /// Starts `true`, matching `Session::role`'s default and for its reason: on
    /// an ordinary local deck the handshake confirms rather than corrects, and
    /// starting refused would flash the hint away and back.
    can_drive: bool,
    /// The deck's keys, held for as long as the dialog is open. `None` while it
    /// is closed; dropping it is what gives them back.
    keys: Option<ModalKeys>,
}

impl Picker {
    /// Wires the dialog that [`WasmElement::render`] wrote.
    fn new(
        dialog: HtmlDialogElement,
        grid: Element,
        input: HtmlInputElement,
        count: Element,
        status: Element,
        thumbs: Rc<RefCell<Thumbnails>>,
    ) -> Self {
        Self {
            dialog,
            grid,
            input,
            count,
            status,
            toggle: None,
            cells: Vec::new(),
            entries: Vec::new(),
            matches: Vec::new(),
            selected: 0,
            current: 0,
            thumbs,
            corpus: Corpus::default(),
            can_drive: true,
            keys: None,
        }
    }

    /// What the photographs currently say.
    ///
    /// Read through a scope of its own so the borrow is over before the caller
    /// touches the DOM: `refresh` runs from inside the very probe that writes
    /// this cell.
    fn previews(&self) -> (Readiness, u32) {
        let thumbs = self.thumbs.borrow();
        (thumbs.readiness(), thumbs.version())
    }

    /// The button that opens the picker, so its `aria-expanded` can be kept true.
    fn set_toggle(&mut self, toggle: Option<Element>) {
        self.toggle = toggle;
    }

    /// Whether this client may move the deck, and says so if it may not.
    ///
    /// The same treatment the presenter's navigation gets, for the same reason:
    /// a control that does nothing is worse than no control. Without it an
    /// audience client's picker looked exactly like a speaker's — it closed on
    /// a click, the deck did not move, and the explanation arrived as a toast
    /// the closing dialog was covering.
    fn set_can_drive(&mut self, can_drive: bool) {
        self.can_drive = can_drive;
        let role = if can_drive { "presenter" } else { "audience" };
        let _ = self.dialog.set_attribute("data-role", role);
        // The grid is still a listbox worth reading; what it is not is a control.
        let _ = self
            .grid
            .set_attribute("aria-disabled", if can_drive { "false" } else { "true" });
        if let Ok(Some(hint)) = self.dialog.query_selector(".strip-hint")
            && !can_drive
        {
            hint.set_text_content(Some(AUDIENCE_HINT));
        }
    }

    /// Moves the deck to `index`, and closes only if the deck will take it.
    ///
    /// Both ways in go through here — a click on a cell and `Enter` on the
    /// selection — because they used to disagree about the same impossible
    /// action: with no channel a click did nothing and left the dialog open
    /// while `Enter` did nothing and shut it.
    fn jump_to(&mut self, index: usize, commands: Option<&UnboundedSender<Command>>) {
        // Refused here as well as in `handle_actions`, which is the one place
        // that owns the rule — but which can only answer into a toast this
        // dialog would be closing over. Staying open is half the message, and
        // the hint bar is the other half.
        if !self.can_drive {
            return;
        }
        let Some(commands) = commands else {
            notify(ToastType::Error, "The slide picker cannot move the deck.");
            return;
        };
        if !jump(commands, index) {
            notify(ToastType::Error, "The deck did not take the jump.");
            return;
        }
        self.close();
    }

    fn is_open(&self) -> bool {
        self.dialog.open()
    }

    /// Marks which cell the deck is on, and brings it into view.
    ///
    /// Scrolled only while the picker is open: scrolling a hidden element moves
    /// it to a position the speaker never asked for, and the next open would
    /// start halfway down the deck.
    fn set_current(&mut self, current: usize) {
        self.current = current;
        let open = self.is_open();
        for (index, cell) in self.cells.iter().enumerate() {
            if index == current {
                let _ = cell.set_attribute("aria-current", "true");
                if open {
                    cell.scroll_into_view();
                }
            } else {
                let _ = cell.remove_attribute("aria-current");
            }
        }
    }

    /// Drops the searchable text, because the deck it described is gone.
    ///
    /// Called when a reload lands, *before* the new outline is asked for — and
    /// the ask may fail, which `app.rs` logs and carries on from by design. What
    /// is left then has to be no corpus rather than the previous deck's: a
    /// picker searching text one reload out of date answers a speaker's query
    /// with a slide that no longer says what they searched for, and jumps them
    /// there confidently. Losing the search is the documented cost of a failed
    /// `/api/outline`; answering it wrong is not.
    fn forget_outline(&mut self) {
        self.entries.clear();
        self.corpus = Corpus::Pending;
    }

    /// The deck's words are not coming: `GET /api/outline` failed.
    ///
    /// Distinct from [`Picker::forget_outline`], which is the same empty corpus
    /// with a request still outstanding. Only the count can tell them apart, and
    /// only this one is worth telling the speaker about — "still loading" that
    /// never resolves is the message that wastes their time mid-talk.
    fn note_outline_failed(&mut self) {
        self.entries.clear();
        self.corpus = Corpus::Failed;
        // Repaint, in case the speaker is looking at it: the ask outlives the
        // keystroke that opened the picker.
        self.apply_query();
    }

    /// Takes the deck's searchable text, and captions the cells with it.
    fn set_outline(&mut self, slides: &[SlideOutline]) {
        self.entries = slides
            .iter()
            .map(|slide| {
                let part = slide.part.as_deref().unwrap_or_default();
                let haystack = fold(&format!(
                    "{} {part} {} {}",
                    slide.title, slide.text, slide.notes
                ));
                Entry {
                    folded_body: fold(&slide.text),
                    folded_notes: fold(&slide.notes),
                    title: slide.title.clone(),
                    body: slide.text.clone(),
                    notes: slide.notes.clone(),
                    haystack,
                }
            })
            .collect();
        self.corpus = Corpus::Loaded;
        // Sized to the outline, not to `max(cells, entries)`: a cell with no
        // entry matches every query (see `apply_query`), inflates the count and
        // sends a `GoTo` past the end of the deck when it is clicked. If the two
        // ever disagree, the outline is the list that knows how long the deck is.
        self.build(self.entries.len());
    }

    /// Builds or trims the grid so there is exactly one cell per slide, and
    /// leaves the filter, the count and the selection describing it.
    fn build(&mut self, total: usize) {
        while self.cells.len() > total {
            if let Some(cell) = self.cells.pop() {
                let _ = self.grid.remove_child(&cell);
            }
        }
        while self.cells.len() < total {
            let index = self.cells.len();
            let Some(cell) = self.make_cell(index) else {
                // Stop building, but still fall through: leaving early would
                // leave `matches`, the count and the selection describing a grid
                // that no longer exists, and `Enter` would jump somewhere the
                // speaker never selected.
                break;
            };
            self.cells.push(cell);
        }
        self.caption_cells();
        // Rather than assigning `matches` directly: the cells that a previous
        // query hid are still hidden, the count still reads against the old
        // deck, and `selected` still points into the old match list. This
        // re-hides, recounts and re-selects in one place.
        self.apply_query();
    }

    /// One cell, carrying its own slide index for [`install_jump`] to read back.
    ///
    /// The index is written into the DOM rather than captured in a closure per
    /// cell, so a forty-slide deck installs one listener instead of forty — and
    /// so rebuilding the grid on a live reload cannot leave the old ones behind.
    ///
    /// A `role="option"` div rather than a `<button>`: the grid is a listbox,
    /// the selection is carried by `aria-activedescendant` from the search
    /// input, and a button inside a listbox is not something a screen reader is
    /// allowed to make sense of.
    fn make_cell(&mut self, index: usize) -> Option<Element> {
        let cell = create_html_element("div");
        cell.set_class_name("strip-cell");
        let _ = cell.set_attribute("role", "option");
        let _ = cell.set_attribute("id", &cell_id(index));
        let _ = cell.set_attribute("data-slide", &index.to_string());
        let _ = cell.set_attribute("aria-selected", "false");
        // 1-based, the way every other number the speaker reads is.
        let number = index + 1;
        cell.set_inner_html(&format!(
            r#"<img alt="" loading="lazy"><span class="strip-number">{number}</span><span class="strip-caption"></span><span class="strip-snippet"></span>"#
        ));

        if let Err(err) = self.grid.append_child(&cell) {
            log_dom_error("append a picker cell", &err);
            return None;
        }
        Some(cell.unchecked_into())
    }

    /// Writes each cell's title under its picture.
    fn caption_cells(&self) {
        for (index, cell) in self.cells.iter().enumerate() {
            let Ok(Some(caption)) = cell.query_selector(".strip-caption") else {
                continue;
            };
            let title = self.entries.get(index).map(|entry| entry.title.as_str());
            caption.set_text_content(title);
            let label = match title {
                Some(title) if !title.is_empty() => format!("Slide {}: {title}", index + 1),
                _ => format!("Slide {}", index + 1),
            };
            let _ = cell.set_attribute("aria-label", &label);
        }
    }

    /// Shows the picker if it is closed, closes it if it is not.
    fn toggle(&mut self) {
        if self.is_open() {
            self.close();
        } else {
            self.show();
        }
    }

    fn show(&mut self) {
        if let Err(err) = self.dialog.show_modal() {
            log_dom_error("open the slide picker", &err);
            // And to the speaker, not only to the console. This runs from a key
            // they just pressed or a button they just clicked, and a gesture
            // that produces no visible effect and no message is a silent failure
            // however well it is logged — they would press it again, and again.
            notify(ToastType::Error, "The slide picker would not open.");
            return;
        }
        // Before anything else it draws: from here until `note_closed` every
        // key on the page is the picker's, so an arrow cannot reach the deck
        // through the gap.
        self.keys = Some(claim_keys_for_modal());
        if let Some(toggle) = &self.toggle {
            let _ = toggle.set_attribute("aria-expanded", "true");
        }
        self.refresh();
        // Opened on the slide the deck is on rather than on the last search: a
        // picker opened mid-talk is a speaker asking "where am I", and the query
        // they typed four slides ago is not an answer.
        self.input.set_value("");
        self.apply_query();
        self.select_slide(self.current);
        // Not a `let _`: a picker whose box is not focused is one the speaker
        // has to click before they can type, which is the whole of what `g` was
        // for. The deck is safe either way now — the claim above does not answer
        // on focus — so this is a usability failure rather than a jump into the
        // room, and it is still worth naming.
        if let Err(err) = self.input.focus() {
            log_dom_error("focus the slide picker's search box", &err);
        }
        self.set_current(self.current);
    }

    fn close(&mut self) {
        // Also the path `Escape` takes, which fires `close` without going
        // through here — see [`install_close`], which is where the toggle's
        // `aria-expanded` is put back.
        self.release_keys();
        self.dialog.close();
    }

    /// Gives the deck its keys back, and the focus with them.
    ///
    /// Idempotent, and called from every point that knows the dialog is on its
    /// way out rather than from the `close` event alone: that event is *queued*,
    /// so between the dialog going invisible and the listener running there is a
    /// window in which the picker is gone and still holds the keyboard. A
    /// speaker who presses `Escape` and an arrow in one movement loses the arrow
    /// to it — which is how this was found, in a browser test that pressed the
    /// two as fast as a browser can.
    ///
    /// The blur is not a tidy-up either, for the same reason it is not one in
    /// [`crate::release_keyboard`]. A `<dialog>` does move focus back when it
    /// closes, but not before the next keystroke can be delivered — so `Escape`
    /// followed straight away by `g` reached `typing_into_editable` while this
    /// input was still on the composed path, was read as a character being
    /// typed, and did not reopen the picker.
    fn release_keys(&mut self) {
        self.keys = None;
        let _ = self.input.blur();
    }

    /// Keeps the button in step with a dialog that may have closed itself, and
    /// gives the deck its keys back.
    ///
    /// Every way out lands here — `Escape`, the ✕, a jump, a host closing it —
    /// because it hangs off the `close` event rather than off [`Picker::close`],
    /// which two of those four never call.
    fn note_closed(&mut self) {
        self.release_keys();
        if let Some(toggle) = &self.toggle {
            let _ = toggle.set_attribute("aria-expanded", "false");
        }
    }

    /// Points every cell at its thumbnail, or says why it cannot.
    fn refresh(&mut self) {
        let (readiness, version) = self.previews();
        match readiness {
            Readiness::Ready => {
                self.status.set_text_content(None);
                for (index, cell) in self.cells.iter().enumerate() {
                    let Ok(Some(image)) = cell.query_selector("img") else {
                        continue;
                    };
                    // A new generation gets a fresh verdict: the file that was
                    // missing last time may have been written this time, and a
                    // cell left marked would stay dimmed for the whole talk.
                    let _ = cell.remove_attribute("data-shot");
                    let _ = image.set_attribute("src", &thumbnail_src(index, version));
                }
            }
            // No pictures to show. Drop the ones that are there rather than
            // leaving them: after a reload they are the *previous* deck's
            // photographs, and the browser will not re-fetch a `src` it already
            // has — so the grid would show the old deck under the new
            // numbering, and a speaker jumping by picture would land elsewhere.
            // `refresh_next` clears its image for the same reason.
            Readiness::Unknown | Readiness::Unavailable => {
                let message = match readiness {
                    Readiness::Unavailable => "Slide previews are unavailable.",
                    _ => "Rendering slide previews…",
                };
                self.status.set_text_content(Some(message));
                for cell in &self.cells {
                    let Ok(Some(image)) = cell.query_selector("img") else {
                        continue;
                    };
                    let _ = image.remove_attribute("src");
                }
            }
        }
        let state = match readiness {
            Readiness::Ready => "ready",
            Readiness::Unknown => "pending",
            Readiness::Unavailable => "unavailable",
        };
        let _ = self.dialog.set_attribute("data-previews", state);
    }

    /// Filters the grid down to the slides the query names.
    ///
    /// Every whitespace-separated token has to appear somewhere in the slide —
    /// its title, its part, its body or its notes — which is what makes a query
    /// of two half-remembered words useful. Accents and case are folded, because
    /// a speaker searching their own French deck mid-talk types neither.
    fn apply_query(&mut self) {
        let query = fold(&self.input.value());
        let tokens = query.split_whitespace().collect::<Vec<_>>();

        self.matches.clear();
        for (index, cell) in self.cells.iter().enumerate() {
            let entry = self.entries.get(index);
            let hit = match entry {
                Some(entry) => matches_query(&entry.haystack, &tokens),
                // No corpus: the cell stays, because a grid that still shows the
                // deck and still jumps is worth having without search. What it
                // must not do is *report* this as a match — the query was never
                // applied to anything. The count below says so instead.
                None => true,
            };
            if hit {
                self.matches.push(index);
                let _ = cell.remove_attribute("hidden");
            } else {
                let _ = cell.set_attribute("hidden", "");
            }
            let snippet = match (hit, entry) {
                (true, Some(entry)) => snippet_for(entry, &tokens),
                _ => None,
            };
            write_snippet(cell, snippet.as_ref());
        }

        let total = self.cells.len();
        let text = match (tokens.is_empty(), self.corpus) {
            (true, _) => format!("{total} slides"),
            (false, Corpus::Loaded) => format!("{} of {total}", self.matches.len()),
            // Never "{total} of {total}". Every cell is showing because there
            // was nothing to filter against, and a count is read as an answer:
            // a speaker typing a word their deck does not contain and being told
            // "40 of 40" concludes the deck contains it forty times over. The
            // one number here is honest — that is how long the deck is — and the
            // clause after it is why it is the only one.
            (false, Corpus::Pending) => format!("{total} slides · still loading the text"),
            (false, Corpus::Failed) => format!("{total} slides · search unavailable"),
        };
        self.count.set_text_content(Some(&text));
        let searchable = match self.corpus {
            Corpus::Loaded => "ready",
            Corpus::Pending => "pending",
            Corpus::Failed => "unavailable",
        };
        let _ = self.dialog.set_attribute("data-search", searchable);

        self.select(0);
    }

    /// Moves the selection by `delta` places, stopping at both ends.
    ///
    /// Clamped rather than wrapped: the speaker is reading a grid, and a
    /// selection that jumps from the last slide back to the first is a
    /// selection they have to go looking for.
    fn move_selection(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let last = self.matches.len() - 1;
        let target = isize::try_from(self.selected)
            .unwrap_or(0)
            .saturating_add(delta)
            .clamp(0, isize::try_from(last).unwrap_or(0));
        self.select(usize::try_from(target).unwrap_or(0));
    }

    fn select_first(&mut self) {
        self.select(0);
    }

    fn select_last(&mut self) {
        self.select(self.matches.len().saturating_sub(1));
    }

    /// How many cells the grid puts on a row, so `↑`/`↓` move a row.
    ///
    /// Read from the resolved `grid-template-columns` rather than computed from
    /// widths: the track list is what the browser actually laid out, and the
    /// grid is `auto-fill`, so nothing here knows the count in advance.
    fn row_len(&self) -> usize {
        window()
            .get_computed_style(&self.grid)
            .ok()
            .flatten()
            .and_then(|style| style.get_property_value("grid-template-columns").ok())
            .map(|tracks| tracks.split_whitespace().count())
            .filter(|count| *count > 0)
            .unwrap_or(1)
    }

    /// The slide the selection is on, for `Enter`.
    fn selected_slide(&self) -> Option<usize> {
        self.matches.get(self.selected).copied()
    }

    /// Selects the `position`-th match, or nothing when the query left none.
    fn select(&mut self, position: usize) {
        self.selected = position.min(self.matches.len().saturating_sub(1));
        for cell in &self.cells {
            let _ = cell.set_attribute("aria-selected", "false");
        }
        let Some(cell) = self
            .matches
            .get(self.selected)
            .and_then(|index| self.cells.get(*index))
        else {
            let _ = self.input.remove_attribute("aria-activedescendant");
            return;
        };
        let _ = cell.set_attribute("aria-selected", "true");
        if let Some(id) = cell.get_attribute("id") {
            let _ = self.input.set_attribute("aria-activedescendant", &id);
        }
        if self.is_open() {
            cell.scroll_into_view();
        }
    }

    /// Puts the selection on a given slide, if the query left it visible.
    fn select_slide(&mut self, index: usize) {
        if let Some(position) = self.matches.iter().position(|slide| *slide == index) {
            self.select(position);
        }
    }
}

/// The id a cell is addressed by from `aria-activedescendant`.
fn cell_id(index: usize) -> String {
    format!("slide-cell-{index}")
}

/// Whether a folded haystack answers a folded query.
///
/// Every token, not any: two half-remembered words are a *narrowing*, and a
/// query that widened as the speaker typed more of what they remembered would
/// be useless. An empty query matches everything, which is what makes the
/// unfiltered grid fall out of the same path as a filtered one.
fn matches_query(haystack: &str, tokens: &[&str]) -> bool {
    tokens.iter().all(|token| haystack.contains(token))
}

/// Lowercase, and Latin accents removed.
///
/// One character in, one character out, so a position in a folded string is the
/// same position in the original — which is what lets a snippet be cut out of
/// the text the speaker wrote rather than out of the folded copy. That rules out
/// the expanding folds (`ß` → `ss`), so the ligatures below fold to their first
/// letter; a deck is searched by someone who knows what is in it.
fn fold_char(ch: char) -> char {
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    match lower {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'æ' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'œ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

fn fold(text: &str) -> String {
    text.chars().map(fold_char).collect()
}

/// A window of the slide's text around the first hit, cut into the part before
/// the hit, the hit, and the part after it — so the hit can be marked without
/// any of the three ever being written as markup.
struct Snippet {
    before: String,
    hit: String,
    after: String,
}

/// The snippet a cell should show, or `None` when the query is already answered
/// by the title the cell shows anyway.
fn snippet_for(entry: &Entry, tokens: &[&str]) -> Option<Snippet> {
    let token = tokens.first()?;
    if token.is_empty() || fold(&entry.title).contains(token) {
        return None;
    }
    [
        (&entry.body, &entry.folded_body),
        (&entry.notes, &entry.folded_notes),
    ]
    .into_iter()
    .find_map(|(text, folded)| window_around(text, folded, token))
}

/// `SNIPPET_CHARS` of `text` around the first occurrence of `token`.
///
/// `folded` is `text` under [`fold_char`], passed in rather than computed
/// because this runs for every matching cell on every keystroke, and on the
/// first character typed nearly every slide matches.
///
/// Cut by character position, which [`fold_char`] guarantees is the same
/// position in the folded copy that was searched and in the text the author
/// wrote.
fn window_around(text: &str, folded: &str, token: &str) -> Option<Snippet> {
    let at = find_chars(folded, token)?;
    let chars = text.chars().collect::<Vec<_>>();
    // A little room before the hit, so the word is read in a phrase rather than
    // at the start of a line, and the rest after it.
    let start = at.saturating_sub(SNIPPET_CHARS / 3);
    let end = (start + SNIPPET_CHARS).min(chars.len());
    let hit_end = (at + token.chars().count()).min(end);

    let cut = |from: usize, to: usize| {
        chars
            .get(from..to.max(from))
            .unwrap_or_default()
            .iter()
            .collect::<String>()
    };
    let mut before = cut(start, at);
    if start > 0 {
        before.insert(0, '…');
    }
    let mut after = cut(hit_end, end);
    if end < chars.len() {
        after.push('…');
    }
    Some(Snippet {
        before,
        hit: cut(at, hit_end),
        after,
    })
}

/// Where `needle` starts in `haystack`, counted in characters rather than bytes.
fn find_chars(haystack: &str, needle: &str) -> Option<usize> {
    let at = haystack.find(needle)?;
    Some(haystack[..at].chars().count())
}

/// Puts a snippet under a cell's caption, with the hit marked.
///
/// Assembled from text nodes and a `<mark>`, never as markup: this is a slide's
/// own words, and a deck that writes `<img onerror=…>` in its notes would
/// otherwise have written it into the page showing the deck.
fn write_snippet(cell: &Element, snippet: Option<&Snippet>) {
    let Ok(Some(element)) = cell.query_selector(".strip-snippet") else {
        return;
    };
    element.set_text_content(None);
    let Some(snippet) = snippet else {
        return;
    };
    let mark = create_html_element("mark");
    mark.set_text_content(Some(&snippet.hit));
    let _ = element.append_child(&document().create_text_node(&snippet.before));
    let _ = element.append_child(&mark);
    let _ = element.append_child(&document().create_text_node(&snippet.after));
}

/// Sends the jump a click on a cell means, and closes the picker behind it.
///
/// One delegated listener on the grid rather than one per cell: the cells are
/// rebuilt whenever the deck reloads, and per-cell closures would have to be
/// dropped in step with them or go on answering for elements no longer in the
/// document.
///
/// Closing is the point. The picker covers the whole view, so leaving it up
/// after a jump hides the very slide the speaker jumped to — along with the
/// notes they jumped to it to read.
fn install_jump(
    picker: &Rc<RefCell<Picker>>,
    commands: Option<UnboundedSender<Command>>,
) -> EventListener {
    let grid = picker.borrow().grid.clone();
    let picker = Rc::clone(picker);
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

        picker.borrow_mut().jump_to(index, commands.as_ref());
    })
}

/// Keeps the toggle button honest however the dialog was closed — the `✕`, a
/// jump, or the `Escape` the platform handles without asking us.
fn install_close(picker: &Rc<RefCell<Picker>>) -> EventListener {
    let dialog = picker.borrow().dialog.clone();
    let picker = Rc::clone(picker);
    EventListener::new(&dialog, "close", move |_| {
        picker.borrow_mut().note_closed();
    })
}

/// Marks a cell whose picture would not load, and says which one.
///
/// The readiness probe asks about slide 0 and speaks for all of them, which is
/// right — they are made in one pass — but it cannot see a single missing
/// `thumb-NNNN.png`. Without this, that cell draws the browser's broken-image
/// glyph for the length of the talk and says nothing about which slide it was.
///
/// Capture rather than bubble: `error` on an `<img>` does not bubble, so a
/// delegated listener has to be told to see it on the way down. One listener
/// for the grid, for the reason [`install_jump`] takes one.
fn install_shot_errors(picker: &Rc<RefCell<Picker>>) -> EventListener {
    let grid = picker.borrow().grid.clone();
    EventListener::new_with_options(
        &grid,
        "error",
        gloo::events::EventListenerOptions::run_in_capture_phase(),
        move |event| {
            let Some(image) = event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
            else {
                return;
            };
            if image.tag_name() != "IMG" {
                return;
            }
            let Ok(Some(cell)) = image.closest(".strip-cell") else {
                return;
            };
            let slide = cell.get_attribute("data-slide").unwrap_or_default();
            error!("No picture for slide", slide);
            let _ = cell.set_attribute("data-shot", "failed");
            // So the cell falls back to its number rather than to a glyph.
            let _ = image.remove_attribute("src");
        },
    )
}

/// Re-filters as the speaker types.
fn install_search(picker: &Rc<RefCell<Picker>>) -> EventListener {
    let input = picker.borrow().input.clone();
    let picker = Rc::clone(picker);
    EventListener::new(&input, "input", move |_| {
        picker.borrow_mut().apply_query();
    })
}

/// Moves the selection, and jumps on `Enter`.
///
/// On the dialog rather than on `window`, so it only ever sees keys typed into
/// the picker — and it `preventDefault`s them, because `↑`/`↓` in a text field
/// move the caret and `Enter` in a form submits.
///
/// It does *not* stop them propagating, and does not need to: the deck's own
/// handler sits on `window` and stands down for as long as the dialog holds its
/// [`claim_keys_for_modal`] guard. That matters most for the keys this match
/// does not name — `space`, the digits, `PageUp` — which fall through to the
/// deck's listener rather than to any arm here, and so cannot be protected by
/// anything written inside this closure.
fn install_navigation(
    picker: &Rc<RefCell<Picker>>,
    commands: Option<UnboundedSender<Command>>,
) -> EventListener {
    let dialog = picker.borrow().dialog.clone();
    let handle = Rc::clone(picker);
    EventListener::new_with_options(
        &dialog,
        "keydown",
        gloo::events::EventListenerOptions::enable_prevent_default(),
        move |event| {
            let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            if event.ctrl_key() || event.meta_key() || event.alt_key() {
                return;
            }
            let mut picker = handle.borrow_mut();
            let row = isize::try_from(picker.row_len()).unwrap_or(1);
            // `Home`/`End` in a text field belong to the caret. The arrows have
            // the same conflict, but there the grid's meaning is the one the
            // speaker wants; for these two, taking them would leave no way to
            // reach either end of a mistyped query.
            let in_query = event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
                .is_some_and(|element| element == **picker.input);
            match event.key().as_str() {
                "ArrowRight" => picker.move_selection(1),
                "ArrowLeft" => picker.move_selection(-1),
                "ArrowDown" => picker.move_selection(row),
                "ArrowUp" => picker.move_selection(-row),
                "Home" if !in_query => picker.select_first(),
                "End" if !in_query => picker.select_last(),
                "Enter" => {
                    let Some(index) = picker.selected_slide() else {
                        return;
                    };
                    picker.jump_to(index, commands.as_ref());
                }
                // Not `preventDefault`ed: the dialog closes itself on `Escape`,
                // and `install_close` hears it. Only the keyboard cannot wait
                // for that — see [`Picker::release_keys`].
                "Escape" => {
                    picker.release_keys();
                    return;
                }
                // Everything else is the query being typed.
                _ => return,
            }
            event.prevent_default();
        },
    )
}

/// The three keys that open the picker.
///
/// Caught here rather than added to [`crate::KeyboardMapping`], which is a table
/// of what the *deck* does: these three open a surface a page chose to mount, so
/// they arrive and leave with it rather than being bound whether or not there is
/// anything for them to open. The help dialog names them beside the deck's own
/// keys, because both pages that run the app mount a picker.
///
/// Bare `g` or `/`, and `Ctrl`/`Cmd`+`K` — the three a speaker reaches for. `g`
/// and `/` are both unbound in the deck's keymap, and a modified key never
/// reaches it at all, so none of the three is taken from the deck.
fn install_keys(picker: &Rc<RefCell<Picker>>, resume: Rc<dyn Fn()>) -> EventListener {
    let picker = Rc::clone(picker);
    let options = gloo::events::EventListenerOptions::enable_prevent_default();
    EventListener::new_with_options(&window(), "keydown", options, move |event| {
        let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
            return;
        };
        let modified = event.ctrl_key() || event.meta_key();
        let opens = match event.key().as_str() {
            "g" | "/" => !modified && !event.alt_key(),
            "k" => modified && !event.alt_key(),
            _ => false,
        };
        // Only when nothing already owns the keyboard, the same guard the
        // deck's own keymap applies for the same reason: a `g` at the quake
        // terminal's prompt is a letter, not a command.
        //
        // An open picker owns it too, so these three go quiet while it is up and
        // `g` is simply typed into the query — which is what they did anyway
        // whenever the box had focus. `Escape` and the ✕ are the ways out, and
        // the hint bar says so.
        if !opens || crate::deck_keys_captured() || crate::typing_into_editable(event) {
            return;
        }
        // `Ctrl+K` and `/` are the browser's own before they are ours.
        event.prevent_default();
        // Before the dialog, and outside the borrow it takes: opening the picker
        // is the moment to ask again about photographs a probe gave up waiting
        // for, and the probe's redraw borrows the same cell.
        resume();
        picker.borrow_mut().toggle();
    })
}

/// A probe's verdict, spent on one picker's grid.
fn redraw_for(picker: &Rc<RefCell<Picker>>) -> Redraw {
    let picker = Rc::clone(picker);
    Rc::new(move || picker.borrow_mut().refresh())
}

/// Sends the jump, and says whether it went.
///
/// A closed channel means the action loop has ended — the client is dead, not
/// merely refused — so the caller has something to tell the speaker and a reason
/// not to close on it.
fn jump(commands: &UnboundedSender<Command>, index: usize) -> bool {
    let command = Command::GoTo {
        slide: SlideId::new(index),
    };
    match commands.unbounded_send(command) {
        Ok(()) => true,
        Err(err) => {
            error!(
                "The slide picker could not send a jump to slide",
                index,
                err.to_string()
            );
            false
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// An entry the way [`Picker::set_outline`] builds one, so a test and the
    /// running picker cannot disagree about how the haystack is joined.
    fn entry(title: &str, part: &str, body: &str, notes: &str) -> Entry {
        Entry {
            haystack: fold(&format!("{title} {part} {body} {notes}")),
            folded_body: fold(body),
            folded_notes: fold(notes),
            title: title.to_owned(),
            body: body.to_owned(),
            notes: notes.to_owned(),
        }
    }

    /// The speaker types neither accents nor capitals mid-talk, and the deck is
    /// full of both.
    #[test]
    fn folding_removes_accents_and_case() {
        assert_eq!(fold("Le Début"), "le debut");
        assert_eq!(fold("ÉÈÊË ÇA ÔÙ"), "eeee ca ou");
        // One char in, one char out — the invariant `window_around` slices by.
        for text in ["Le Début", "ÆØ œuf", "Straße", "ÿÑ"] {
            assert_eq!(
                fold(text).chars().count(),
                text.chars().count(),
                "folding {text:?} changed its length"
            );
        }
    }

    /// Every token has to appear, not any: more of what the speaker remembers
    /// must narrow the grid rather than widen it.
    #[test]
    fn a_query_narrows_as_tokens_are_added() {
        let haystack = fold("Ownership Le borrow checker et les durees de vie");

        assert!(matches_query(&haystack, &["borrow"]));
        assert!(matches_query(&haystack, &["borrow", "checker"]));
        // Second token absent — an `any` would still call this a match.
        assert!(!matches_query(&haystack, &["borrow", "kubernetes"]));
        // An empty query is the unfiltered grid.
        assert!(matches_query(&haystack, &[]));
    }

    /// A word typed without its accents finds the slide that spells it with
    /// them, and the snippet is cut out of what the *author* wrote.
    #[test]
    fn a_snippet_is_cut_from_the_unfolded_text() {
        let text = "Le café est chaud";
        let snippet = window_around(text, &fold(text), "cafe").expect("a hit");

        // The accent survives into what is shown, though it was searched folded.
        assert_eq!(snippet.hit, "café");
        assert!(text.contains(&snippet.before.replace('…', "")));
        assert!(text.contains(&snippet.after.replace('…', "")));
    }

    /// Byte offsets and character offsets diverge the moment a deck is French,
    /// and a snippet cut at the wrong one splits a character.
    #[test]
    fn positions_are_counted_in_characters() {
        // "é" is two bytes: a byte offset here would be 6, a char offset 5.
        assert_eq!(find_chars("café x", "x"), Some(5));
        assert_eq!(find_chars("abc", "z"), None);
    }

    /// The caption already answers the query, so a snippet repeating it would
    /// spend the cell's second line saying nothing new.
    #[test]
    fn a_title_match_gets_no_snippet() {
        let slide = entry("Ownership", "", "le borrow checker", "");
        assert!(snippet_for(&slide, &["ownership"]).is_none());
    }

    /// The whole point of searching the notes: the speaker remembers what they
    /// meant to *say*, and the slide itself does not contain the word.
    #[test]
    fn a_notes_only_match_is_snippeted_from_the_notes() {
        let slide = entry(
            "Ownership",
            "",
            "le borrow checker",
            "insister sur les lifetimes",
        );
        let snippet = snippet_for(&slide, &["lifetimes"]).expect("a hit in the notes");
        assert_eq!(snippet.hit, "lifetimes");
    }

    /// A query that left nothing selects nothing, rather than falling back to
    /// the first slide — which `Enter` would then jump to.
    #[test]
    fn no_matches_means_no_selection() {
        let slide = entry("Ownership", "", "le borrow checker", "");
        assert!(!matches_query(&slide.haystack, &["kubernetes"]));
        assert!(snippet_for(&slide, &["kubernetes"]).is_none());
    }
}
