use gloo::console::error;
use gloo::utils::document;
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
    // whole job is to hide the deck.
    let _ = overlay.set_attribute(
        "style",
        &format!(
            "position:fixed;inset:0;z-index:2147483646;background:{}",
            screen.color()
        ),
    );
    if body.append_child(&overlay).is_err() {
        error!("Failed to blank the screen");
    }
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
