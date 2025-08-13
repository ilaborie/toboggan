use gloo::console::error;
use wasm_bindgen::JsValue;

/// Common DOM error types
#[derive(Debug)]
pub enum DomError {
    ElementCreation(String),
    ElementNotFound(String),
    PropertySet(String),
    Append(String),
}

impl From<JsValue> for DomError {
    fn from(js_val: JsValue) -> Self {
        let message = js_val
            .as_string()
            .unwrap_or_else(|| "Unknown DOM error".to_string());
        DomError::ElementCreation(message)
    }
}

/// Log and ignore DOM errors (for operations where failure should not panic)
pub fn log_dom_error(operation: &str, error: &JsValue) {
    let error_msg = error
        .as_string()
        .unwrap_or_else(|| "Unknown error".to_string());
    error!("DOM operation failed:", operation, "Error:", error_msg);
}

/// Macro for DOM operations that should log errors but not panic
#[macro_export]
macro_rules! dom_try {
    ($operation:expr, $op_name:expr) => {
        if let Err(err) = $operation {
            $crate::utils::errors::log_dom_error($op_name, &err);
        }
    };
}

/// Macro for DOM operations that return a Result and should continue on error
#[macro_export]
macro_rules! dom_try_or_continue {
    ($operation:expr, $op_name:expr) => {
        match $operation {
            Ok(val) => val,
            Err(e) => {
                $crate::utils::errors::log_dom_error($op_name, &e);
                continue;
            }
        }
    };
}

/// Macro for DOM operations that return a Result and should return on error
#[macro_export]
macro_rules! dom_try_or_return {
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
