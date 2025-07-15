use gloo::utils::document;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, Node, ShadowRoot, ShadowRootInit, ShadowRootMode};

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

pub(crate) fn create_and_append_element<T>(parent: &Node, tag: &str) -> Result<T, JsValue>
where
    T: JsCast + Clone,
{
    let element = document().create_element(tag)?;
    parent.append_child(&element)?;
    Ok(element.dyn_into::<T>()?)
}

pub trait StateClassMapper<T> {
    fn to_css_class(&self) -> &'static str;
}
