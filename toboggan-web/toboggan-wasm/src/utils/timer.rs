use std::{cell::Cell, rc::Rc};

/// RAII timer that automatically starts console.time on creation and console.timeEnd on drop
pub struct Timer {
    label: &'static str,
    is_active: Rc<Cell<bool>>,
}

impl Timer {
    /// Try to create a new timer. Returns None if a timer with the same label is already active.
    pub fn new(label: &'static str, is_active: Rc<Cell<bool>>) -> Option<Self> {
        if is_active.get() {
            // Timer already active, don't create a new one
            None
        } else {
            is_active.set(true);
            web_sys::console::time_with_label(label);
            Some(Self { label, is_active })
        }
    }
    
    /// Try to end a timer if it's currently active
    pub fn try_end(is_active: &Rc<Cell<bool>>, label: &'static str) {
        if is_active.get() {
            is_active.set(false);
            web_sys::console::time_end_with_label(label);
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if self.is_active.get() {
            self.is_active.set(false);
            web_sys::console::time_end_with_label(self.label);
        }
    }
}