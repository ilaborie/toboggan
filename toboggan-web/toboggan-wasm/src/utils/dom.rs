use gloo::console::error;
use gloo::utils::document;
use toboggan_core::{Content, Style};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};

fn escape_html(html: &str) -> String {
    let div = document().create_element("div").unwrap_throw();
    div.set_text_content(Some(html));
    div.inner_html()
}

#[must_use]
pub fn create_html_element(tag: &str) -> HtmlElement {
    let result = document().create_element(tag).unwrap_throw();
    result.dyn_into().unwrap_throw()
}

#[must_use]
pub fn render_content(content: &Content, wrapper: Option<&str>) -> String {
    let inner = match content {
        Content::Empty => String::new(),
        Content::Text { text } => escape_html(text),
        Content::Html { raw, .. } => raw.clone(),
    };

    if let Some(wrapper) = wrapper {
        format!("<{wrapper}>{inner}</{wrapper}>",)
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
    let temp = document().create_element("div").unwrap_throw();
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
