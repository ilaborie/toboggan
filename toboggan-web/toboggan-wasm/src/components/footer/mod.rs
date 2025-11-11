use web_sys::{Element, HtmlElement};

use crate::components::WasmElement;
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try};

const CSS: &str = include_str!("style.css");

#[derive(Debug, Default)]
pub struct TobogganFooterElement {
    container: Option<Element>,
    content: Option<String>,
}

impl TobogganFooterElement {
    pub fn set_content(&mut self, content: Option<String>) {
        self.content = content;
        self.render_content();
    }

    fn render_content(&mut self) {
        let Some(container) = &self.container else {
            return;
        };

        let html = self.content.as_deref().unwrap_or("");
        container.set_inner_html(html);
    }
}

impl WasmElement for TobogganFooterElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create shadow root"
        );

        let container: Element = dom_try!(
            create_and_append_element(&root, "footer"),
            "create footer element"
        );

        self.container = Some(container);
        self.render_content();
    }
}
