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
use crate::create_html_element;

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
