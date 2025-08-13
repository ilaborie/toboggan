use web_sys::{Element, HtmlElement, ShadowRoot};

use toboggan_core::Slide;

use crate::{
    components::WasmElement, 
    render_content,
    create_shadow_root_with_style,
    create_and_append_element,
    unwrap_or_return,
    dom_try_or_return
};

#[derive(Debug, Default)]
pub struct TobogganSlideElement {
    parent: Option<HtmlElement>,
    root: Option<ShadowRoot>,
    container: Option<Element>,
    slide: Option<Slide>,
}

impl TobogganSlideElement {
    pub fn set_slide(&mut self, slide: Option<Slide>) {
        self.slide = slide;
        self.render_slide();
    }

    fn render_slide(&mut self) {
        let container = unwrap_or_return!(&self.container);
        
        let content = match &self.slide {
            Some(slide) => {
                let title = render_content(&slide.title, None);
                let body = render_content(&slide.body, Some("article"));
                
                if title.is_empty() {
                    body
                } else {
                    format!("<h2>{title}</h2>{body}")
                }
            }
            None => "<article>Empty slide</article>".to_string(),
        };

        container.set_inner_html(&content);
    }
}

impl WasmElement for TobogganSlideElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try_or_return!(
            create_shadow_root_with_style(host, include_str!("./style.css")),
            "create shadow root"
        );

        let container: Element = dom_try_or_return!(
            create_and_append_element(&root, "section"),
            "create section element"
        );

        self.root = Some(root);
        self.parent = Some(host.clone());
        self.container = Some(container);

        self.render_slide();
    }
}
