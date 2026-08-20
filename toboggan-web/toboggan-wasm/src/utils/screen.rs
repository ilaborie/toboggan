use gloo::console::error;
use gloo::utils::document;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::create_html_element;

/// The colour a blanked screen shows.
///
/// Two of them because they are used for different things: black to take the
/// room's attention off the screen and back onto the speaker, white to turn the
/// projector into a lamp — the trick for a demo on a whiteboard, or for reading
/// faces in a dark room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankScreen {
    Black,
    White,
}

impl BlankScreen {
    const fn color(self) -> &'static str {
        match self {
            Self::Black => "#000",
            Self::White => "#fff",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::White => "white",
        }
    }
}

/// Marks the blanking overlay so it can be found again to take it down.
const BLANK_ID: &str = "toboggan-blank";

/// Covers this tab's screen, or uncovers it.
///
/// Local to the tab. The server has no blank state and none is wanted: blanking
/// is what the projector shows, and a presenter watching their notes in a second
/// client should still see the slide the room has stopped looking at.
///
/// Pressing the *other* blank key while one is up swaps the colour rather than
/// uncovering, which is what someone reaching for white from black means.
pub fn toggle_blank(screen: BlankScreen) {
    let Some(body) = document().body() else {
        error!("No <body> to blank");
        return;
    };

    let Some(existing) = document().get_element_by_id(BLANK_ID) else {
        insert_blank(&body, screen);
        return;
    };

    let same_colour = existing.get_attribute("data-screen").as_deref() == Some(screen.name());
    if body.remove_child(&existing).is_err() {
        error!("Failed to unblank the screen");
        return;
    }
    if !same_colour {
        insert_blank(&body, screen);
    }
}

fn insert_blank(body: &HtmlElement, screen: BlankScreen) {
    let overlay = create_html_element("div");
    overlay.set_id(BLANK_ID);
    let _ = overlay.set_attribute("data-screen", screen.name());
    // Styled inline rather than from a stylesheet: a deck brings its own CSS,
    // and the one element that must win against all of it is the one whose
    // whole job is to hide the deck. The `margin`/`border`/`padding`/size
    // resets are not decoration — the UA sheet dresses every `[popover]` with
    // `width: fit-content` and `margin: auto`, which would shrink the overlay
    // to a dot in the middle of the screen.
    let _ = overlay.set_attribute(
        "style",
        &format!(
            "position:fixed;inset:0;width:auto;height:auto;margin:0;border:0;padding:0;\
             overflow:hidden;z-index:2147483646;background:{}",
            screen.color()
        ),
    );
    if body.append_child(&overlay).is_err() {
        error!("Failed to blank the screen");
        return;
    }
    show_in_top_layer(&overlay);
    // The badge is meant to sit above the blanking overlay, and the top layer
    // stacks in show order rather than by z-index — so a badge that was already
    // up has to be shown again to get back on top of the overlay.
    raise_goto_badge();
}

/// Puts `overlay` in the top layer, where the deck's own chrome has to be.
///
/// A z-index is no longer enough to be on top: a maximized terminal is a
/// `popover`, and the top layer paints above every z-index there is. The
/// inline z-indexes are kept for the case where `showPopover` is unavailable,
/// which leaves the old stacking order intact.
///
/// Position inside the top layer is show order, so anything that has to sit
/// above something already there must be shown after it.
fn show_in_top_layer(overlay: &HtmlElement) {
    if overlay.set_attribute("popover", "manual").is_err() {
        error!("Failed to put an overlay in the top layer");
        return;
    }
    if let Err(err) = overlay.show_popover() {
        error!(
            "Failed to show an overlay in the top layer:",
            format!("{err:?}")
        );
        let _ = overlay.remove_attribute("popover");
    }
}

/// Re-shows the goto badge, so it sits above whatever has just joined the top
/// layer. Does nothing when no badge is up.
fn raise_goto_badge() {
    let Some(badge) = document()
        .get_element_by_id(GOTO_ID)
        .and_then(|badge| badge.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let _ = badge.hide_popover();
    show_in_top_layer(&badge);
}

/// Marks the badge showing the slide number being typed.
const GOTO_ID: &str = "toboggan-goto";

/// Sits above the blanking overlay, deliberately: a presenter who has blanked
/// the screen to take a question is exactly the one about to jump somewhere,
/// and they still need to see what they are typing.
/// `top`/`left`/`margin`/`border` are resets, not layout: the UA sheet's
/// `[popover]` rule sets `inset: 0` and `margin: auto`, which would drag the
/// badge into the middle of the screen and draw a border around it.
const GOTO_STYLE: &str = "position:fixed;top:auto;left:auto;bottom:1.5rem;right:1.5rem;\
margin:0;border:0;z-index:2147483647;\
padding:.3rem .8rem;border-radius:.4rem;background:rgba(0,0,0,.78);color:#fff;\
font:600 1.4rem/1.2 system-ui,sans-serif";

/// Shows the slide number being typed, or takes the badge away.
///
/// Without it the digits go nowhere visible: the presenter types `1`, then `2`,
/// watches nothing happen, and has no way to tell whether the deck is listening
/// or whether they have already jumped somewhere.
pub fn show_goto_pending(number: Option<usize>) {
    let Some(body) = document().body() else {
        return;
    };
    let existing = document().get_element_by_id(GOTO_ID);

    let Some(number) = number else {
        if let Some(badge) = existing {
            let _ = body.remove_child(&badge);
        }
        return;
    };

    let badge = if let Some(badge) = existing {
        badge
    } else {
        let badge = create_html_element("div");
        badge.set_id(GOTO_ID);
        let _ = badge.set_attribute("style", GOTO_STYLE);
        if body.append_child(&badge).is_err() {
            error!("Failed to show the slide number being typed");
            return;
        }
        show_in_top_layer(&badge);
        badge.into()
    };
    badge.set_text_content(Some(&format!("→ {number}")));
}

/// Puts the page into fullscreen, or takes it out.
///
/// Must be called from inside the keydown handler: browsers only grant
/// fullscreen off a user gesture, and handing the request to a task that runs
/// later loses it.
pub fn toggle_fullscreen() {
    let document = document();
    if document.fullscreen_element().is_some() {
        document.exit_fullscreen();
        return;
    }
    let Some(root) = document.document_element() else {
        return;
    };
    if root.request_fullscreen().is_err() {
        error!("The browser refused to go fullscreen");
    }
}
