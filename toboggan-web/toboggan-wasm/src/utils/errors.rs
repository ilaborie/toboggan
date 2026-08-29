use gloo::console::error;
use wasm_bindgen::{JsCast as _, JsValue};

/// Log DOM errors without panicking
pub fn log_dom_error(operation: &str, error: &JsValue) {
    error!(
        "DOM operation failed:",
        operation,
        "Error:",
        describe(error)
    );
}

/// The most informative string a rejected DOM call can be reduced to.
///
/// `JsValue::as_string` answers only for a JS *string*, and a DOM call rejects
/// with a `DOMException` object — so reading it alone reported every failure in
/// this crate as "Unknown error", discarding the one part worth having. Whether
/// `showModal` threw `InvalidStateError` (already open) or `NotSupportedError`
/// (the element is not in the document) is the whole diagnosis, and both looked
/// identical in the console.
fn describe(error: &JsValue) -> String {
    if let Some(text) = error.as_string() {
        return text;
    }
    if let Some(exception) = error.dyn_ref::<js_sys::Error>() {
        return format!("{}: {}", exception.name(), exception.message());
    }
    // Not a string and not an `Error`: `String(value)`, which is what a console
    // would have shown, rather than giving up.
    String::from(js_sys::JsString::from(error.clone()))
}

/// Simplified macro for DOM operations with error logging
#[macro_export]
macro_rules! dom_try {
    ($operation:expr, $op_name:expr) => {
        match $operation {
            Ok(val) => val,
            Err(err) => {
                $crate::utils::errors::log_dom_error($op_name, &err);
                return;
            }
        }
    };
}

/// Simplified macro for safe Option unwrapping
#[macro_export]
macro_rules! unwrap_or_return {
    ($option:expr) => {
        match $option {
            Some(val) => val,
            None => return,
        }
    };
}
