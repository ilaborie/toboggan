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
        Content::Html { raw, .. } => raw.to_string(),
        Content::Grid { style, cells } => {
            let Style { classes, .. } = style;
            let classes = classes.join(" ");
            let body = cells
                .iter()
                .map(|content| render_content(content, None))
                .collect::<String>();
            format!(r#"<div class="cell {classes}">{body}</div>"#)
        }
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
