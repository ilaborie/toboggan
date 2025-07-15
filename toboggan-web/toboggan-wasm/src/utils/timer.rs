/// RAII timer that automatically measures performance
pub struct Timer {
    label: &'static str,
}

impl Timer {
    #[must_use]
    pub fn new(label: &'static str) -> Self {
        web_sys::console::time_with_label(label);
        Self { label }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        web_sys::console::time_end_with_label(self.label);
    }
}
