use toboggan_core::{Slide, SlideKind};
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use crate::components::WasmElement;
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try, render_content};

const CSS: &str = include_str!("style.css");

#[derive(Debug, Default)]
pub struct TobogganSlideElement {
    container: Option<Element>,
    slide: Option<Slide>,
}

impl TobogganSlideElement {
    pub fn set_slide(&mut self, slide: Option<Slide>, current_step: usize) {
        self.slide = slide;
        self.render_slide();
        self.set_current_step(current_step);
    }

    /// Set the current step state on the DOM.
    /// `step` represents how many steps have been revealed (0 = none, 1 = first step visible, etc.)
    pub fn set_current_step(&self, step: usize) {
        let Some(container) = &self.container else {
            return;
        };

        let Ok(steps) = container.query_selector_all(".step") else {
            return;
        };

        for i in 0..steps.length() {
            if let Some(node) = steps.item(i)
                && let Ok(element) = node.dyn_into::<Element>()
            {
                let class_name = element.class_name();
                let mut new_classes: Vec<&str> = class_name
                    .split_whitespace()
                    .filter(|c| *c != "step-done" && *c != "step-current")
                    .collect();

                let step_index = i as usize;
                if step_index < step {
                    new_classes.push("step-done");
                    // Mark the last revealed step as current
                    if step_index + 1 == step {
                        new_classes.push("step-current");
                    }
                }

                element.set_class_name(&new_classes.join(" "));
            }
        }
    }

    fn render_slide(&mut self) {
        let Some(container) = &self.container else {
            return;
        };

        let content = if let Some(slide) = &self.slide {
            // Apply style classes and add slide kind class
            let mut classes = slide.style.classes.clone();

            // Add slide kind as CSS class
            let kind_class = match slide.kind {
                SlideKind::Cover => "cover",
                SlideKind::Part => "part",
                SlideKind::Standard => "standard",
            };
            classes.push(kind_class.to_string());

            let class_string = classes.join(" ");
            container.set_class_name(&class_string);

            // Apply inline style if present
            if let Some(style) = &slide.style.style {
                let _ = container.set_attribute("style", style);
            } else {
                let _ = container.remove_attribute("style");
            }

            let title = render_content(&slide.title, None);
            let body = render_content(&slide.body, Some("article"));

            if title.is_empty() {
                body
            } else {
                format!("<h2>{title}</h2>{body}")
            }
        } else {
            // Clear any previous styles
            container.set_class_name("");
            let _ = container.remove_attribute("style");
            "<article></article>".to_string()
        };

        container.set_inner_html(&content);
    }
}

impl WasmElement for TobogganSlideElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create shadow root"
        );

        let container: Element = dom_try!(
            create_and_append_element(&root, "section"),
            "create section element"
        );

        self.container = Some(container);
        self.render_slide();
    }
}
