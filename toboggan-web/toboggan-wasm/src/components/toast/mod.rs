use std::fmt::Display;

use gloo::events::EventListener;
use gloo::timers::callback::Timeout;
use gloo::utils::document;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

use crate::components::WasmElement;
use crate::{create_and_append_element, create_shadow_root_with_style, dom_try};

const CSS: &str = include_str!("style.css");

#[derive(Debug, Clone, Copy)]
pub enum ToastType {
    Error,
    Warning,
    Info,
    Success,
}

impl Display for ToastType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(fmt, "error"),
            Self::Warning => write!(fmt, "warning"),
            Self::Info => write!(fmt, "info"),
            Self::Success => write!(fmt, "success"),
        }
    }
}

#[derive(Debug)]
pub struct TobogganToastElement {
    container: Option<HtmlElement>,
    duration_ms: u32,
}

impl Default for TobogganToastElement {
    fn default() -> Self {
        Self {
            container: None,
            duration_ms: 3000,
        }
    }
}

impl TobogganToastElement {
    pub fn toast(&self, toast_type: ToastType, message: &str) {
        let Some(container) = &self.container else {
            return;
        };

        let toast = dom_try!(
            document()
                .create_element("output")
                .map(|el| el.dyn_into::<HtmlElement>().unwrap_throw()),
            "create toast element"
        );

        dom_try!(toast.set_attribute("role", "status"), "set role");
        toast.set_class_name(&toast_type.to_string());
        toast.set_inner_html(&format!(
            r#"<p>{message}</p><button class="close" title="close"></button>"#
        ));

        if let Ok(Some(btn)) = toast.query_selector("button") {
            let btn = btn.dyn_into::<HtmlElement>().unwrap_throw();
            let toast_clone = toast.clone();
            let container_clone = container.clone();
            EventListener::new(&btn, "click", move |_| {
                let _ = container_clone.remove_child(&toast_clone);
            })
            .forget();
        }

        Self::animate_toast_entry(container, &toast);
        let _ = container.append_child(&toast);

        let toast_clone = toast.clone();
        Timeout::new(self.duration_ms, move || {
            if let Some(parent) = toast_clone.parent_node() {
                let _ = parent.remove_child(&toast_clone);
            }
        })
        .forget();
    }

    fn animate_toast_entry(container: &HtmlElement, _toast: &HtmlElement) {
        if container.child_element_count() > 0 {
            let first = container.offset_height();
            let last = container.offset_height() + 50; // Estimate
            let invert = last - first;

            if invert != 0 {
                let style = container.style();
                style
                    .set_property("transform", &format!("translateY({invert}px)"))
                    .ok();
                style
                    .set_property("transition", "transform 150ms ease-out")
                    .ok();
                let _ = container.offset_height(); // Force reflow
                style.set_property("transform", "translateY(0)").ok();

                let container_clone = container.clone();
                Timeout::new(200, move || {
                    container_clone.style().set_property("transition", "").ok();
                })
                .forget();
            }
        }
    }
}

impl WasmElement for TobogganToastElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create shadow root"
        );

        let container: HtmlElement = dom_try!(
            create_and_append_element(&root, "footer"),
            "create footer element"
        );

        self.container = Some(container);
    }
}
