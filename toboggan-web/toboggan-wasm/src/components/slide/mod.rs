use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

use toboggan_core::{Slide, SlideKind};

use crate::components::WasmElement;
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try, render_content};

const CSS: &str = include_str!("style.css");

#[derive(Debug, Default)]
pub struct TobogganSlideElement {
    container: Option<Element>,
    slide: Option<Slide>,
}

impl TobogganSlideElement {
    pub fn set_slide(&mut self, slide: Option<Slide>) {
        self.slide = slide;
        self.render_slide();
        self.reset_steps();
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

    fn reset_steps(&self) {
        let Some(container) = &self.container else {
            return;
        };

        let steps = container.query_selector_all(".step").ok();
        if let Some(steps) = steps {
            for i in 0..steps.length() {
                if let Some(step) = steps.item(i)
                    && let Ok(element) = step.dyn_into::<Element>()
                {
                    let class_name = element.class_name();
                    let new_classes = class_name
                        .split_whitespace()
                        .filter(|content| *content != "step-done" && *content != "step-current")
                        .collect::<Vec<_>>()
                        .join(" ");
                    element.set_class_name(&new_classes);
                }
            }
        }
    }

    fn update_current_step(&self) {
        let Some(container) = &self.container else {
            return;
        };

        // Remove step-current from all steps
        if let Ok(steps) = container.query_selector_all(".step") {
            for i in 0..steps.length() {
                if let Some(step) = steps.item(i)
                    && let Ok(element) = step.dyn_into::<Element>()
                {
                    let class_name = element.class_name();
                    let new_classes = class_name
                        .split_whitespace()
                        .filter(|content| *content != "step-current")
                        .collect::<Vec<_>>()
                        .join(" ");
                    element.set_class_name(&new_classes);
                }
            }
        }

        // Add step-current to the last step-done
        if let Ok(done_steps) = container.query_selector_all(".step.step-done")
            && done_steps.length() > 0
        {
            let last_index = done_steps.length() - 1;
            if let Some(step) = done_steps.item(last_index)
                && let Ok(element) = step.dyn_into::<Element>()
            {
                let class_name = element.class_name();
                element.set_class_name(&format!("{class_name} step-current"));
            }
        }
    }

    pub fn next_step(&mut self) -> bool {
        let Some(container) = &self.container else {
            return false;
        };

        let steps = match container.query_selector_all(".step") {
            Ok(steps) if steps.length() > 0 => steps,
            _ => return false,
        };

        for i in 0..steps.length() {
            if let Some(step) = steps.item(i)
                && let Ok(element) = step.dyn_into::<Element>()
            {
                let class_name = element.class_name();
                if !class_name.contains("step-done") {
                    element.set_class_name(&format!("{class_name} step-done"));
                    self.update_current_step();
                    return true;
                }
            }
        }

        // All steps are done
        false
    }

    pub fn previous_step(&mut self) -> bool {
        let Some(container) = &self.container else {
            return false;
        };

        let done_steps = match container.query_selector_all(".step.step-done") {
            Ok(steps) if steps.length() > 0 => steps,
            _ => return false,
        };

        let last_index = done_steps.length() - 1;
        if let Some(step) = done_steps.item(last_index)
            && let Ok(element) = step.dyn_into::<Element>()
        {
            let class_name = element.class_name();
            let new_classes = class_name
                .split_whitespace()
                .filter(|content| *content != "step-done")
                .collect::<Vec<_>>()
                .join(" ");
            element.set_class_name(&new_classes);
            self.update_current_step();
            return true;
        }

        false
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
