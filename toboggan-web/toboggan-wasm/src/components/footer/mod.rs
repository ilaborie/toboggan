use web_sys::{Element, HtmlElement, ShadowRoot};

use crate::components::WasmElement;
use crate::{
    create_and_append_element, create_shadow_root_with_style, dom_try_or_return, unwrap_or_return,
};

#[derive(Debug, Default)]
pub struct TobogganFooterElement {
    parent: Option<HtmlElement>,
    root: Option<ShadowRoot>,
    container: Option<Element>,
    content: Option<String>,
}

impl TobogganFooterElement {
    pub fn set_content(&mut self, content: Option<String>) {
        self.content = content;
        self.render_content();
    }

    fn render_content(&mut self) {
        let container = unwrap_or_return!(&self.container);

        // Set content or leave empty (CSS will handle default content via ::before)
        if let Some(content) = &self.content {
            container.set_text_content(Some(content));
        } else {
            container.set_inner_html("");
        }
    }
}

impl WasmElement for TobogganFooterElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try_or_return!(
            create_shadow_root_with_style(host, include_str!("./style.css")),
            "create shadow root"
        );

        let container: Element = dom_try_or_return!(
            create_and_append_element(&root, "footer"),
            "create footer element"
        );

        self.root = Some(root);
        self.parent = Some(host.clone());
        self.container = Some(container);

        self.render_content();
    }
}
