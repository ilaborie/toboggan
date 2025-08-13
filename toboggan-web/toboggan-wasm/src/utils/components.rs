use gloo::utils::document;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlButtonElement, HtmlElement, Node, ShadowRoot, ShadowRootInit, ShadowRootMode};

/// Creates a shadow root with embedded CSS styles
pub(crate) fn create_shadow_root_with_style(
    host: &HtmlElement,
    css: &str,
) -> Result<ShadowRoot, JsValue> {
    let shadow_mode = ShadowRootInit::new(ShadowRootMode::Open);
    let root = host.attach_shadow(&shadow_mode)?;

    let style_el = document().create_element("style")?;
    style_el.set_text_content(Some(css));
    root.append_child(&style_el)?;

    Ok(root)
}

/// Creates and appends an element to a parent, returning the typed element
pub(crate) fn create_and_append_element<T>(parent: &Node, tag: &str) -> Result<T, JsValue>
where
    T: JsCast + Clone,
{
    let element = document().create_element(tag)?;
    parent.append_child(&element)?;
    Ok(element.dyn_into::<T>()?)
}

/// Creates a styled button element
pub(crate) fn create_button(icon: &str, title: &str) -> Result<HtmlButtonElement, JsValue> {
    let btn = document()
        .create_element("button")?
        .dyn_into::<HtmlButtonElement>()?;
    btn.set_text_content(Some(icon));
    btn.set_title(title);
    Ok(btn)
}

/// Macro for safe Option unwrapping with early return
#[macro_export]
macro_rules! unwrap_or_return {
    ($option:expr) => {
        match $option {
            Some(val) => val,
            None => return,
        }
    };
}

/// Macro for safe Option unwrapping with early return and custom return value
#[macro_export]
macro_rules! unwrap_or_return_with {
    ($option:expr, $return_val:expr) => {
        match $option {
            Some(val) => val,
            None => return $return_val,
        }
    };
}

/// Component state mapper for CSS classes
pub trait StateClassMapper<T> {
    fn to_css_class(&self) -> &'static str;
}
