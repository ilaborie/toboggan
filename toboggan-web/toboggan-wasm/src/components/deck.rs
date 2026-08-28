//! The DOM the room looks at.
//!
//! `/run` builds it, and each of the presenter view's mirrors builds it again
//! inside its own iframe. It lives here rather than in either caller because a
//! mirror that lays the deck out differently from the deck is worse than no
//! mirror at all — and that is exactly what the presenter view used to do, by
//! rendering the slide component on its own without the footer, the wrapper or
//! the state classes the deck puts around it.

use wasm_bindgen::UnwrapThrowExt as _;
use web_sys::HtmlElement;

use crate::components::{TobogganFooterElement, TobogganSlideElement, WasmElement};
use crate::services::mirror::MirrorFrame;
use crate::{create_html_element, inject_head_html, set_document_lang};

/// The classes [`apply_deck_state`] owns. It replaces its own and leaves every
/// other class on the host alone, because the host also carries whatever the
/// page that built it put there.
const STATE_CLASSES: [&str; 3] = ["init", "running", "done"];

/// Builds the deck under `host`: the slide, and the footer beneath it.
pub(crate) fn mount_deck(
    host: &HtmlElement,
    slide: &mut TobogganSlideElement,
    footer: &mut TobogganFooterElement,
) {
    let el = create_html_element("div");
    el.set_class_name("toboggan-slide");
    slide.render(&el);
    host.append_child(&el).unwrap_throw();

    let el = create_html_element("footer");
    el.set_class_name("toboggan-footer");
    footer.render(&el);
    host.append_child(&el).unwrap_throw();
}

/// Writes the deck's position onto `host`: the `init`/`running`/`done` class
/// `state.css` animates from, and the two counters a deck's own CSS reads to
/// number its slides.
///
/// Takes the class and the number rather than the [`toboggan_core::State`] they
/// come from, because one caller has no state to give. The presenter view's
/// next-slide pane shows a slide the deck has not reached: there is a class it
/// should render under and a number it should show, but no state that is true
/// of it.
pub(crate) fn apply_deck_state(
    host: &HtmlElement,
    state_class: &str,
    current_slide: usize,
    total_slides: usize,
) {
    let current_classes = host.class_name();
    let classes = current_classes
        .split_whitespace()
        .filter(|class| !STATE_CLASSES.contains(class))
        .collect::<Vec<_>>();

    let new_classes = if classes.is_empty() {
        state_class.to_owned()
    } else {
        format!("{} {state_class}", classes.join(" "))
    };
    host.set_class_name(&new_classes);

    let style = host.style();
    let _ = style.set_property("--current-slide", &current_slide.to_string());
    let _ = style.set_property("--total-slides", &total_slides.to_string());
}

/// A document that *is* the deck, and the pieces of it a frame touches.
///
/// Two pages are one: a presenter mirror, painted by `postMessage` from the
/// window that frames it, and a shot page, painted once from the REST API for a
/// screenshot. Neither opens a socket, neither registers a client, and both owe
/// the room the same layout — so the painting belongs here, beside
/// [`mount_deck`], rather than in either of them.
pub(crate) struct DeckPainter {
    host: HtmlElement,
    slide: TobogganSlideElement,
    footer: TobogganFooterElement,
    /// The `_head.html` already in this document.
    ///
    /// Compared before injecting, because the presenter posts a frame on every
    /// reveal and `inject_head_html` removes and re-adds everything it manages:
    /// re-injecting an unchanged head would tear the deck's stylesheet out of
    /// the document and put it back on every press of the space bar, which is a
    /// frame of unstyled slide each time.
    head: Option<String>,
}

impl DeckPainter {
    /// Builds the deck under `host` and returns the painter for it.
    pub(crate) fn mount(host: &HtmlElement) -> Self {
        let mut slide = TobogganSlideElement::default();
        // Terminals belong to the deck the room is watching. A second set here
        // would be a second set of shells, in a second session, showing output
        // nobody asked for — and in the presenter's next-slide pane, for a slide
        // nobody has reached yet.
        slide.set_preview(true);
        let mut footer = TobogganFooterElement::default();
        mount_deck(host, &mut slide, &mut footer);

        Self {
            host: host.clone(),
            slide,
            footer,
            head: None,
        }
    }

    /// Paints one frame.
    ///
    /// Head first, so the slide is laid out against the deck's own fonts rather
    /// than reflowing once they arrive.
    pub(crate) fn show(&mut self, frame: MirrorFrame) {
        let MirrorFrame {
            head,
            footer,
            lang,
            slide,
            step,
            state_class,
            slide_number,
            total_slides,
        } = frame;

        if self.head != head {
            inject_head_html(head.as_deref());
            self.head = head;
        }
        set_document_lang(lang.as_deref());
        self.footer.set_content(footer);
        apply_deck_state(&self.host, &state_class, slide_number, total_slides);
        // `None` asks for every reveal at once — the presenter's next-slide pane
        // and every thumbnail — which is what the slide component spells
        // `usize::MAX`.
        self.slide.set_slide(slide, step.unwrap_or(usize::MAX));
    }
}
