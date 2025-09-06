use gloo::console::error;
use wasm_bindgen::JsValue;

/// Log DOM errors without panicking
pub fn log_dom_error(operation: &str, error: &JsValue) {
    let error_msg = error
        .as_string()
        .unwrap_or_else(|| "Unknown error".to_string());
    error!("DOM operation failed:", operation, "Error:", error_msg);
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
