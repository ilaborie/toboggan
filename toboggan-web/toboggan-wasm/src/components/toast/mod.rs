use std::cell::RefCell;
use std::fmt::Display;

use gloo::events::EventListener;
use gloo::timers::callback::Timeout;
use gloo::utils::document;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, ShadowRoot};

use crate::components::WasmElement;
use crate::{
    create_and_append_element, create_shadow_root_with_style, dom_try, dom_try_or_return,
    unwrap_or_return,
};

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
    parent: Option<HtmlElement>,
    root: Option<ShadowRoot>,
    container: Option<HtmlElement>,
    // Store event listeners and timeouts to prevent memory leaks
    event_listeners: RefCell<Vec<EventListener>>,
    timeouts: RefCell<Vec<Timeout>>,
    /// Duration in milliseconds for how long a toast should be visible (default: 3000ms)
    pub duration_ms: u32,
}

impl Default for TobogganToastElement {
    fn default() -> Self {
        Self {
            parent: None,
            root: None,
            container: None,
            event_listeners: RefCell::new(Vec::new()),
            timeouts: RefCell::new(Vec::new()),
            duration_ms: 3000, // 3 seconds default
        }
    }
}

impl TobogganToastElement {
    pub fn toast(&self, toast_type: ToastType, message: &str) {
        let container = unwrap_or_return!(&self.container);

        let node = dom_try_or_return!(
            document()
                .create_element("output")
                .map(|el| el.dyn_into::<HtmlElement>().unwrap_throw()),
            "create toast element"
        );

        dom_try!(node.set_attribute("role", "status"), "set role attribute");
        node.set_class_name(&toast_type.to_string());

        let inner = format!(
            r#"<p>{message}</p><button class="close" title="close" style="color:inherit;"></button>"#
        );
        node.set_inner_html(&inner);

        // Toast animations implemented in Rust - store returned objects to prevent memory leaks
        if let Some(event_listener) = register_close_handler(container, &node) {
            self.event_listeners.borrow_mut().push(event_listener);
        }
        if let Some(timeout) = toast_animation(container, &node) {
            self.timeouts.borrow_mut().push(timeout);
        }
        let timeout = wait_and_remove(&node, self.duration_ms);
        self.timeouts.borrow_mut().push(timeout);
    }
}

impl WasmElement for TobogganToastElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try_or_return!(
            create_shadow_root_with_style(host, include_str!("./style.css")),
            "create shadow root"
        );

        let container: HtmlElement = dom_try_or_return!(
            create_and_append_element(&root, "footer"),
            "create footer element"
        );

        self.root = Some(root);
        self.parent = Some(host.clone());
        self.container = Some(container);
    }
}

/// Register a close button click handler using gloo `EventListener`
fn register_close_handler(container: &HtmlElement, node: &HtmlElement) -> Option<EventListener> {
    let node_clone = node.clone();
    let container_clone = container.clone();

    if let Ok(btn) = node.query_selector("button")
        && let Some(btn) = btn
    {
        let btn = btn.dyn_into::<HtmlElement>().unwrap_throw();
        let event_listener = EventListener::new(&btn, "click", move |_event| {
            let _ = container_clone.remove_child(&node_clone);
        });
        return Some(event_listener);
    }
    None
}

/// Implement FLIP animation for smooth height transitions using gloo Timeout
fn toast_animation(container: &HtmlElement, node: &HtmlElement) -> Option<Timeout> {
    // Check if container has children (FLIP technique)
    if container.child_element_count() > 0 {
        // FLIP: First - record the current height
        let first = container.offset_height();

        // FLIP: Last - append node and record new height
        let _ = container.append_child(node);
        let last = container.offset_height();

        // FLIP: Invert - calculate the difference
        let invert = last - first;

        if invert != 0 {
            // Use CSS transition for smooth animation (simpler than Web Animations API)
            let container_style = container.style();
            container_style
                .set_property("transform", &format!("translateY({invert}px)"))
                .unwrap_throw();
            container_style
                .set_property("transition", "transform 150ms ease-out")
                .unwrap_throw();

            // Trigger reflow then remove transform to animate
            let _ = container.offset_height(); // Force reflow
            container_style
                .set_property("transform", "translateY(0)")
                .unwrap_throw();

            // Clean up transition after animation using gloo Timeout
            let container_clone = container.clone();
            let timeout = Timeout::new(200, move || {
                // Slightly longer than animation duration
                container_clone
                    .style()
                    .set_property("transition", "")
                    .unwrap_throw();
            });
            return Some(timeout);
        }
    } else {
        // No existing children, just append
        let _ = container.append_child(node);
    }
    None
}

/// Wait for animations to finish and remove the node using gloo Timeout
fn wait_and_remove(node: &HtmlElement, duration_ms: u32) -> Timeout {
    // Configurable timeout-based removal
    let node_clone = node.clone();
    Timeout::new(duration_ms, move || {
        if let Some(parent) = node_clone.parent_node() {
            let _ = parent.remove_child(&node_clone);
        }
    })
}
