use gloo::console::error;
use gloo::utils::{document, window};
use toboggan_core::{Content, Secret, Style};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{AddEventListenerOptions, Element, Event, EventTarget, HtmlElement, KeyboardEvent};

fn escape_html(html: &str) -> String {
    let div = document()
        .create_element("div")
        .expect_throw("DOM unavailable: could not create div for HTML escaping");
    div.set_text_content(Some(html));
    div.inner_html()
}

/// Whether `event` targets an editable control (form field or `contenteditable`),
/// where bare-key shortcuts should defer to normal typing.
///
/// Looks only at `event.target`, which the DOM **retargets** to the shadow host
/// for an event that started inside a shadow root — so this deliberately does
/// not see the terminal's hidden textarea, and backtick keeps toggling the quake
/// overlay while the shell has focus. Use [`typing_into_editable`] where the
/// question is "is the user typing", not "what did the event name".
#[must_use]
pub fn is_editable_target(event: &KeyboardEvent) -> bool {
    let Some(target) = event.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<HtmlElement>() else {
        return false;
    };
    is_editable_element(&element)
}

/// Whether `event` is the user typing into an editable control, wherever that
/// control lives.
///
/// Unlike [`is_editable_target`] this walks `composedPath()`, which crosses
/// shadow boundaries. That is the difference that matters for the terminals:
/// rioterm reads keystrokes from a hidden textarea inside the terminal's shadow
/// root, so from a listener on `window` the event's target is the shadow host —
/// an ordinary `<div>`. Judging by the target alone, typing `space` at a shell
/// prompt looked exactly like pressing `space` on a slide.
#[must_use]
pub fn typing_into_editable(event: &KeyboardEvent) -> bool {
    event
        .composed_path()
        .iter()
        .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
        .any(|element| is_editable_element(&element))
}

fn is_editable_element(element: &HtmlElement) -> bool {
    element.is_content_editable()
        || matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
}

/// Releases keyboard focus when a click lands outside the focused widget.
///
/// A terminal keeps focus in a hidden textarea, and the deck's keys are
/// deliberately inert while it does. Browsers only move focus when the click
/// lands on something focusable, and a slide is a plain `<div>` — so without
/// this, clicking off a slide's terminal left it holding the keyboard and the
/// presenter with a deck that no longer answered its arrow keys and no obvious
/// way out. The quake overlay has its own exit (its toggle key, or a click
/// outside it); an inline terminal had none.
///
/// `document.activeElement` is already retargeted to the outermost shadow host,
/// which is exactly the granularity wanted here: a click anywhere inside the
/// focused terminal — its canvas, its title bar — keeps the focus, and only a
/// click outside it releases.
pub fn install_focus_release_on_outside_click() {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        let Some(focused) = document().active_element() else {
            return;
        };
        if focused.tag_name() == "BODY" {
            return;
        }
        let focused: &EventTarget = focused.as_ref();
        let inside = event
            .composed_path()
            .iter()
            .any(|node| node.dyn_ref::<EventTarget>() == Some(focused));
        if !inside {
            blur_active_element();
        }
    });

    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    if document()
        .add_event_listener_with_callback_and_add_event_listener_options(
            "click",
            closure.as_ref().unchecked_ref(),
            &options,
        )
        .is_err()
    {
        error!("Failed to register focus-release listener");
    }
    closure.forget();
}

/// Blurs whatever currently has focus, descending through shadow roots.
///
/// `document.activeElement` stops at a shadow host, so a plain `blur()` on it
/// leaves the real target — rioterm's hidden textarea, several roots down —
/// still focused. Closing the quake terminal has to actually give focus back:
/// while that textarea holds it, [`typing_into_editable`] rightly reports the
/// user as typing and the deck's keys stay inert.
pub fn blur_active_element() {
    let mut active = document().active_element();
    while let Some(element) = active {
        let next = element.shadow_root().and_then(|root| root.active_element());
        if next.is_none() {
            if let Ok(html) = element.dyn_into::<HtmlElement>() {
                let _ = html.blur();
            }
            return;
        }
        active = next;
    }
}

/// Installs a page-lifetime capture-phase `keydown` listener on `window`.
///
/// Capture phase lets the handler run before slide/terminal key handlers (which
/// otherwise consume the key). The closure is intentionally leaked via `forget`
/// because the listener lives for the whole page and is never removed.
pub fn install_capture_keydown(mut handler: impl FnMut(&KeyboardEvent) + 'static) {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: KeyboardEvent| handler(&event));
    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    window()
        .add_event_listener_with_callback_and_add_event_listener_options(
            "keydown",
            closure.as_ref().unchecked_ref(),
            &options,
        )
        .unwrap_throw();
    closure.forget();
}

#[must_use]
pub fn create_html_element(tag: &str) -> HtmlElement {
    let result = document()
        .create_element(tag)
        .expect_throw("DOM unavailable: could not create element");
    result
        .dyn_into()
        .expect_throw("create_element returned a non-HtmlElement node")
}

#[must_use]
pub fn render_content(content: &Content, wrapper: Option<&str>) -> String {
    let inner = match content {
        Content::Empty => String::new(),
        Content::Text { text } => escape_html(text),
        Content::Html { raw, .. } => raw.clone(),
    };

    if let Some(wrapper) = wrapper {
        format!("<{wrapper}>{inner}</{wrapper}>")
    } else {
        inner
    }
}

pub fn apply_slide_styles(container: &Element, style: &Style) {
    // Apply CSS classes
    if style.classes.is_empty() {
        container.set_class_name("");
    } else {
        let classes = style.classes.join(" ");
        container.set_class_name(&classes);
    }
}

/// The presenter token this page was opened with, if any.
///
/// Read from `?token=` on the page's own URL, because that is where the server
/// prints it in the presenter link — one string to copy onto a phone or a
/// second laptop, rather than a field to fill in. A page opened without one is
/// an audience member, which is the right default for a link shared with a
/// room.
///
/// Decoding is [`Secret::from_query_value`]'s, not `decodeURIComponent`'s. This
/// used to call the latter, which leaves `+` alone where the server reads it as
/// a space, so a token containing one arrived as different text than was sent
/// and the presenter was silently demoted.
///
/// Not stored anywhere: the token stays in the URL, so closing the tab forgets
/// it and sharing the *audience* URL cannot leak it by accident.
#[must_use]
pub fn presenter_token() -> Option<Secret> {
    let search = match window().location().search() {
        Ok(search) => search,
        Err(err) => {
            error!("Could not read the page's query string:", err);
            return None;
        }
    };
    let query = search.strip_prefix('?').unwrap_or(&search);
    query
        .split('&')
        .find_map(|pair| Secret::from_query_value(pair.strip_prefix("token=")?))
}

/// Sets the page's language from the deck.
///
/// The served shell is a static `index.html` that can only say `lang="en"`; the
/// deck's own language does not exist until the talk has been fetched. Left at
/// the default, a screen reader reads a French deck aloud with an English voice,
/// and the browser hyphenates it by English rules.
pub fn set_document_lang(lang: Option<&str>) {
    let Some(root) = document().document_element() else {
        return;
    };
    if root.set_attribute("lang", lang.unwrap_or("en")).is_err() {
        error!("Failed to set the page language");
    }
}

/// Injects custom head HTML into document.head
/// Removes any previously injected elements and adds new ones with data-toboggan-head marker
pub fn inject_head_html(head_html: Option<&str>) {
    let Some(head) = document().head() else {
        error!("Could not get document head");
        return;
    };

    // Remove previously injected elements
    let selector = "[data-toboggan-head]";
    if let Ok(existing) = head.query_selector_all(selector) {
        for i in 0..existing.length() {
            if let Some(node) = existing.get(i) {
                let _ = head.remove_child(&node);
            }
        }
    }

    // If no new head HTML, we're done
    let Some(html) = head_html else {
        return;
    };

    // Create temporary container to parse HTML
    let temp = document()
        .create_element("div")
        .expect_throw("DOM unavailable: could not create temp div for head HTML injection");
    temp.set_inner_html(html);

    // Move each child to document.head with marker attribute.
    //
    // `append_child` is also what removes the node from `temp`, so the loop's
    // termination depends on it succeeding. Logging the failure and carrying on
    // left the same node at the front for ever: the tab span, emitting console
    // errors, on any node `<head>` declines — which a deck's own `_head.html`
    // can contain. It is removed either way now.
    while let Some(child) = temp.first_child() {
        if let Some(element) = child.dyn_ref::<Element>() {
            let _ = element.set_attribute("data-toboggan-head", "true");
        }

        if head.append_child(&child).is_err() {
            error!("Could not move an element from _head.html into <head>");
            if temp.remove_child(&child).is_err() {
                error!("Could not drop it either; abandoning the rest of _head.html");
                return;
            }
        }
    }
}
