use gloo::console::error;
use gloo::utils::{document, window};
use toboggan_core::{Content, Style};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{AddEventListenerOptions, Element, HtmlElement, KeyboardEvent};

fn escape_html(html: &str) -> String {
    let div = document()
        .create_element("div")
        .expect_throw("DOM unavailable: could not create div for HTML escaping");
    div.set_text_content(Some(html));
    div.inner_html()
}

/// Whether `event` targets an editable control (form field or `contenteditable`),
/// where bare-key shortcuts should defer to normal typing.
#[must_use]
pub fn is_editable_target(event: &KeyboardEvent) -> bool {
    let Some(target) = event.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<HtmlElement>() else {
        return false;
    };
    if element.is_content_editable() {
        return true;
    }
    matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
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

    // Move each child to document.head with marker attribute
    while let Some(child) = temp.first_child() {
        // Add marker attribute if it's an element
        if let Some(element) = child.dyn_ref::<Element>() {
            let _ = element.set_attribute("data-toboggan-head", "true");
        }

        // Move to head
        if head.append_child(&child).is_err() {
            error!("Failed to append element to head");
        }
    }
}
