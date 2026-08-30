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
//! * **The search box disarms the deck.** [`crate::typing_into_editable`] walks
//!   the event's `composedPath`, so it sees this input through however many
//!   shadow boundaries it is mounted behind: while it has focus the deck's whole
//!   keymap stands down of its own accord, and the arrows, the digits and
//!   `Enter` belong to the picker without anything being taken from the deck.
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
use crate::components::thumbnails::{Readiness, Thumbnails, thumbnail_src};
use crate::utils::errors::log_dom_error;
use crate::{
    create_and_append_element, create_html_element, create_shadow_root_with_style, dom_try,
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
    pub(crate) fn set_toggle(&self, toggle: Option<Element>) {
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

    /// Builds one cell per slide, for a deck whose length is known before its
    /// words are.
    pub(crate) fn build(&self, total: usize) {
        self.with(|picker| picker.build(total));
    }

    /// Marks the cell the deck is on.
    pub(crate) fn set_current(&self, current: usize) {
        self.with(|picker| picker.set_current(current));
    }

    /// Repaints the grid from what the thumbnails now say.
    pub(crate) fn refresh(&self) {
        self.with(Picker::refresh);
    }

    /// Opens the picker, or closes it if it is already open.
    pub(crate) fn toggle(&self) {
        self.with(Picker::toggle);
    }

    /// Runs `action` against the picker, if there is one.
    ///
    /// One place for that `if`, rather than one per method: a picker whose
    /// markup did not come up is a picker the host goes on calling, and every
    /// one of those calls has the same nothing to do.
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

        if let Some(commands) = self.commands.clone() {
            self.listeners.push(install_jump(&picker, commands));
        }
        self.listeners.push(install_close(&picker));
        self.listeners.push(install_search(&picker));
        self.listeners.push(install_shot_errors(&picker));
        self.listeners
            .push(install_navigation(&picker, self.commands.clone()));
        self.listeners.push(install_keys(&picker));
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
struct Picker {
    dialog: HtmlDialogElement,
    grid: Element,
    input: HtmlInputElement,
    count: Element,
    status: Element,
    toggle: Option<Element>,
    /// One cell per presented slide, in deck order.
    cells: Vec<Element>,
    /// The searchable deck. Empty until `GET /api/outline` answers, which costs
    /// the picker its filtering and nothing else.
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
            return;
        }
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
        // Not a `let _`: the whole keymap design rests on this input holding
        // focus. `typing_into_editable` is what stands the deck's keys down,
        // and it answers on focus — so if this rejects, the speaker's next
        // keystrokes drive the *room* instead of the search box, and a digit
        // jumps the deck.
        if let Err(err) = self.input.focus() {
            log_dom_error("focus the slide picker's search box", &err);
        }
        self.set_current(self.current);
    }

    fn close(&mut self) {
        // Also the path `Escape` takes, which fires `close` without going
        // through here — see [`install_close`], which is where the toggle's
        // `aria-expanded` is put back.
        self.dialog.close();
    }

    /// Keeps the button in step with a dialog that may have closed itself.
    fn note_closed(&self) {
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
                // No outline: everything matches, so the picker still shows the
                // deck and still jumps. Only the filtering is gone.
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
        let text = if tokens.is_empty() {
            format!("{total} slides")
        } else {
            format!("{} of {total}", self.matches.len())
        };
        self.count.set_text_content(Some(&text));

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
fn install_jump(picker: &Rc<RefCell<Picker>>, commands: UnboundedSender<Command>) -> EventListener {
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

        jump(&commands, index);
        picker.borrow_mut().close();
    })
}

/// Keeps the toggle button honest however the dialog was closed — the `✕`, a
/// jump, or the `Escape` the platform handles without asking us.
fn install_close(picker: &Rc<RefCell<Picker>>) -> EventListener {
    let dialog = picker.borrow().dialog.clone();
    let picker = Rc::clone(picker);
    EventListener::new(&dialog, "close", move |_| {
        picker.borrow().note_closed();
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
/// the picker — and it stops them there, because `↑`/`↓` in a text field move
/// the caret and `Enter` in a form submits.
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
                    if let Some(commands) = &commands {
                        jump(commands, index);
                    }
                    picker.close();
                }
                // Everything else is the query being typed, `Escape` included:
                // the dialog closes itself, and `install_close` hears it.
                _ => return,
            }
            event.prevent_default();
        },
    )
}

/// The three keys that open the picker.
///
/// Caught here rather than added to [`crate::KeyboardMapping`], because the
/// picker is not something the deck does: a binding in the shared keymap would
/// appear in the deck's own help dialog on every page, naming a surface only the
/// pages that mount this component have. A host's toggle button names all three
/// in its tooltip instead — the dialog's hint bar cannot, since it is only
/// readable once the picker is already open.
///
/// Bare `g` or `/`, and `Ctrl`/`Cmd`+`K` — the three a speaker reaches for. `g`
/// and `/` are both unbound in the deck's keymap, and a modified key never
/// reaches it at all, so none of the three is taken from the deck.
fn install_keys(picker: &Rc<RefCell<Picker>>) -> EventListener {
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
        // Only when nothing is being typed into, the same guard the deck's own
        // keymap applies for the same reason: a `g` at the quake terminal's
        // prompt is a letter, not a command. It is also what makes the picker's
        // own search box safe — `typing_into_editable` reads the event's
        // `composedPath`, so it sees an input inside this shadow root.
        if !opens || crate::deck_keys_captured() || crate::typing_into_editable(event) {
            return;
        }
        // `Ctrl+K` and `/` are the browser's own before they are ours.
        event.prevent_default();
        picker.borrow_mut().toggle();
    })
}

fn jump(commands: &UnboundedSender<Command>, index: usize) {
    let command = Command::GoTo {
        slide: SlideId::new(index),
    };
    if commands.unbounded_send(command).is_err() {
        error!("The slide picker could not send a jump");
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
