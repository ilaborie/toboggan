use gloo::console;
use gloo::events::EventListener;
use gloo::timers::callback::{Interval, Timeout};
use gloo::utils::{document, window};
use toboggan_core::{ClientId, Command, Content, Notification, Renderer, Slide, SlideId, State};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlElement, KeyboardEvent, MessageEvent, WebSocket};

extern crate alloc;
use alloc::boxed::Box;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

// Error types for better error handling
#[derive(Debug)]
pub enum TobogganError {
    WebSocketError(String),
    ParseError(String),
    DomError(String),
    NetworkError(String),
    ConfigError(String),
}

impl From<TobogganError> for JsValue {
    fn from(err: TobogganError) -> Self {
        JsValue::from_str(&format!("{err:?}"))
    }
}

// Configuration for the Toboggan app
#[wasm_bindgen]
#[derive(Clone)]
pub struct TobogganConfig {
    websocket_url: String,
    auto_retry: bool,
    retry_attempts: u32,
    preload_slides: bool,
}

impl Default for TobogganConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl TobogganConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TobogganConfig {
        TobogganConfig {
            websocket_url: "ws://localhost:8080/api/ws".to_string(),
            auto_retry: true,
            retry_attempts: 3,
            preload_slides: true,
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_websocket_url(&mut self, url: String) {
        self.websocket_url = url;
    }

    #[wasm_bindgen(setter)]
    pub fn set_auto_retry(&mut self, auto_retry: bool) {
        self.auto_retry = auto_retry;
    }

    #[wasm_bindgen(setter)]
    pub fn set_retry_attempts(&mut self, attempts: u32) {
        self.retry_attempts = attempts;
    }

    #[wasm_bindgen(setter)]
    pub fn set_preload_slides(&mut self, preload: bool) {
        self.preload_slides = preload;
    }
}

// Set up panic hook for better error messages
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

// Internal app state wrapped in RefCell for safe interior mutability
struct TobogganAppState {
    client_id: ClientId,
    websocket: Option<WebSocket>,
    current_slide: Option<SlideId>,
    slides_cache: Option<SlidesCache>,
    timer: Option<Interval>,
    start_time: Option<f64>,
    presentation_state: Option<State>,
    config: TobogganConfig,
    retry_count: u32,

    // DOM elements
    connection_status: HtmlElement,
    slide_counter: HtmlElement,
    duration_display: HtmlElement,
    error_display: HtmlElement,
    app_element: HtmlElement,

    // Event listeners and closures
    _event_listeners: Vec<EventListener>,
    _closures: Vec<Closure<dyn FnMut()>>,
}

#[wasm_bindgen]
pub struct TobogganApp {
    state: Rc<RefCell<TobogganAppState>>,
}

#[derive(Clone)]
struct SlidesCache {
    slides: Vec<(SlideId, Slide)>,
    total_count: usize,
}

#[wasm_bindgen]
impl TobogganApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<TobogganApp, JsValue> {
        Self::new_with_config(TobogganConfig::new())
    }

    pub fn new_with_config(config: TobogganConfig) -> Result<TobogganApp, JsValue> {
        let client_id = ClientId::new();

        let state = TobogganAppState {
            client_id,
            websocket: None,
            current_slide: None,
            slides_cache: None,
            timer: None,
            start_time: None,
            presentation_state: None,
            config,
            retry_count: 0,
            connection_status: get_element_by_id("connection-status")?,
            slide_counter: get_element_by_id("slide-counter")?,
            duration_display: get_element_by_id("duration-display")?,
            error_display: get_element_by_id("error-display")?,
            app_element: get_element_by_id("app")?,
            _event_listeners: Vec::new(),
            _closures: Vec::new(),
        };

        Ok(TobogganApp {
            state: Rc::new(RefCell::new(state)),
        })
    }

    #[wasm_bindgen]
    pub fn start(&mut self) -> Result<(), JsValue> {
        self.with_error_boundary(|| {
            {
                let state = self.state.borrow();
                state.update_connection_status("Connecting...", "connecting");
            }
            self.connect_with_retry()?;
            self.setup_navigation_buttons()?;
            self.setup_keyboard_handlers()?;
            Ok(())
        })
    }

    #[wasm_bindgen]
    pub fn dispose(&mut self) {
        let mut state = self.state.borrow_mut();

        // Clean up WebSocket
        if let Some(ws) = &state.websocket {
            let _ = ws.close();
        }

        // Clean up timer
        state.stop_timer();

        // Clean up event listeners
        state._event_listeners.clear();

        // Clean up closures to prevent memory leaks
        state._closures.clear();

        // Clear caches
        state.slides_cache = None;
        state.presentation_state = None;
    }

    fn connect_with_retry(&self) -> Result<(), JsValue> {
        let state = self.state.borrow();
        let max_retries = state.config.retry_attempts;
        drop(state);

        for attempt in 1..=max_retries {
            match self.connect() {
                Ok(()) => {
                    self.state.borrow_mut().retry_count = 0;
                    return Ok(());
                }
                Err(_e) if attempt < max_retries => {
                    console::warn!("Connection attempt {} failed, retrying...", attempt);
                    self.state.borrow_mut().retry_count = attempt;

                    // Schedule retry
                    let state_weak = Rc::downgrade(&self.state);
                    let retry_delay = 1000 * attempt; // Exponential backoff

                    let _ = Timeout::new(retry_delay, move || {
                        if let Some(_state_rc) = state_weak.upgrade() {
                            // Retry would need to be implemented differently
                            console::log!("Retrying connection...");
                        }
                    });
                }
                Err(e) => {
                    let state = self.state.borrow_mut();
                    state.show_error_with_category(&TobogganError::WebSocketError(format!(
                        "Failed to connect after {max_retries} attempts"
                    )));
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn connect(&self) -> Result<(), JsValue> {
        let state = self.state.borrow();
        let ws = WebSocket::new(&state.config.websocket_url)?;
        let client_id = state.client_id;

        // Clone elements for closures
        let status_elem = state.connection_status.clone();
        let error_elem = state.error_display.clone();
        drop(state);

        // Create weak reference to state for callbacks
        let state_weak = Rc::downgrade(&self.state);

        // OnOpen handler
        let ws_clone = ws.clone();
        let _state_weak_open = state_weak.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_: Event| {
            console::log!("WebSocket connected");
            status_elem.set_text_content(Some("Connected"));
            status_elem.set_class_name("connected");

            // Register client
            let register_cmd = Command::Register {
                client: client_id,
                renderer: Renderer::Html,
            };
            if let Ok(json) = serde_json::to_string(&register_cmd) {
                let _ = ws_clone.send_with_str(&json);
            }
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // OnMessage handler
        let state_weak_message = state_weak.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                let text_str = String::from(text);
                match serde_json::from_str::<Notification>(&text_str) {
                    Ok(notification) => {
                        console::log!("Received notification:", format!("{notification:?}"));
                        if let Some(state_rc) = state_weak_message.upgrade() {
                            TobogganAppState::handle_notification_static(&state_rc, notification);
                        }
                    }
                    Err(err) => {
                        console::error!("Failed to parse notification:", format!("{err}"));
                        error_elem.set_text_content(Some(&format!(
                            "Failed to parse server message: {err}"
                        )));
                        let css_style = error_elem.style();
                        css_style.set_property("display", "block").unwrap_or(());
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // OnClose handler
        let status_elem_close = self.state.borrow().connection_status.clone();
        let state_weak_close = state_weak.clone();
        let onclose_callback = Closure::wrap(Box::new(move |_: Event| {
            console::log!("WebSocket connection closed");
            status_elem_close.set_text_content(Some("Disconnected"));
            status_elem_close.set_class_name("disconnected");

            if let Some(state_rc) = state_weak_close.upgrade() {
                let mut state = state_rc.borrow_mut();
                state.slides_cache = None;
                state.stop_timer();
            }
        }) as Box<dyn FnMut(Event)>);
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        self.state.borrow_mut().websocket = Some(ws);
        Ok(())
    }

    // Add error boundary wrapper
    fn with_error_boundary<F, R>(&self, operation: F) -> Result<R, JsValue>
    where
        F: FnOnce() -> Result<R, JsValue>,
    {
        operation().map_err(|e| {
            console::error!("Operation failed:", format!("{:?}", e));
            let state = self.state.borrow();
            state.show_error("An internal error occurred");
            e
        })
    }
}

// Implementation for TobogganAppState
impl TobogganAppState {
    fn handle_notification_static(
        state_rc: &Rc<RefCell<TobogganAppState>>,
        notification: Notification,
    ) {
        let mut state = state_rc.borrow_mut();
        match notification {
            Notification::State {
                state: new_state, ..
            } => state.handle_state_notification(new_state),
            Notification::Error { message, .. } => state.show_error(&message),
            Notification::Pong { .. } => console::log!("Received pong"),
        }
    }

    fn handle_state_notification(&mut self, state: State) {
        self.presentation_state = Some(state.clone());

        match &state {
            State::Running {
                current,
                since,
                total_duration,
            } => {
                self.current_slide = Some(*current);
                self.update_connection_status("Running", "running");

                // Calculate start time using jiff timestamp
                let since_ms = since.as_millisecond() as f64;
                let duration_ms = total_duration.as_secs() as f64 * 1000.0
                    + total_duration.subsec_nanos() as f64 / 1_000_000.0;
                self.start_time = Some(since_ms - duration_ms);
                self.start_timer();
            }
            State::Paused {
                current,
                total_duration,
            } => {
                self.current_slide = Some(*current);
                self.update_connection_status("Paused", "paused");
                self.stop_timer();
                self.update_duration_display(total_duration.as_secs());
            }
            State::Done {
                current,
                total_duration,
            } => {
                self.current_slide = Some(*current);
                self.update_connection_status("Done", "done");
                self.stop_timer();
                self.update_duration_display(total_duration.as_secs());
            }
        }

        let _ = self.load_current_slide();
    }

    #[allow(dead_code)]
    fn send_command(&self, command: &Command) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            let json = serde_json::to_string(command)
                .map_err(|e| TobogganError::ParseError(e.to_string()))?;
            console::log!("Sending command:", format!("{command:?}"));
            ws.send_with_str(&json)
                .map_err(|e| TobogganError::WebSocketError(format!("{e:?}")).into())
        } else {
            self.show_error("Not connected to server");
            Ok(())
        }
    }

    fn load_current_slide(&mut self) -> Result<(), JsValue> {
        let slide_id = self.current_slide;
        if let Some(slide_id) = slide_id {
            // If we don't have slides cached, fetch them
            if self.slides_cache.is_none() {
                let _ = self.fetch_slides();
            }

            if let Some(cache) = &self.slides_cache {
                if let Some((_, slide)) = cache.slides.iter().find(|(id, _)| *id == slide_id) {
                    self.display_slide(slide);
                    self.update_slide_counter();
                }
            }
        }
        Ok(())
    }

    fn fetch_slides(&mut self) -> Result<(), JsValue> {
        console::log!("Fetching slides...");

        // For now, create a minimal cache structure
        // This should be replaced with actual API call
        self.slides_cache = Some(SlidesCache {
            slides: vec![], // Would be populated from API
            total_count: 8, // Hardcoded for now
        });

        Ok(())
    }

    fn display_slide(&self, slide: &Slide) {
        let mut content = String::with_capacity(512); // Pre-allocate

        // Add slide title if present
        if let Content::Text { text } = &slide.title {
            content.push_str(&format!("<h2>{}</h2>", escape_html(text)));
        }

        // Add slide body with security
        match &slide.body {
            Content::Text { text } => {
                content.push_str(&format!("<div>{}</div>", escape_html(text)));
            }
            Content::Html { raw, .. } => {
                // Sanitize HTML content for security
                let sanitized = sanitize_html(raw);
                content.push_str(&format!("<div>{sanitized}</div>"));
            }
            Content::IFrame { url } => {
                if self.is_safe_url(url) {
                    content.push_str(&format!(
                        "<div><iframe src=\"{}\" sandbox=\"allow-scripts allow-same-origin\" title=\"Embedded content\"></iframe></div>", 
                        escape_html(url)
                    ));
                } else {
                    content.push_str("<div>Unsafe URL blocked</div>");
                }
            }
            Content::HBox { contents, .. } => {
                content.push_str("<div class=\"hbox\">");
                for item in contents {
                    self.render_content_to_buffer(item, &mut content);
                }
                content.push_str("</div>");
            }
            Content::VBox { contents, .. } => {
                content.push_str("<div class=\"vbox\">");
                for item in contents {
                    self.render_content_to_buffer(item, &mut content);
                }
                content.push_str("</div>");
            }
            Content::Empty => {}
            Content::Term { .. } => {
                content.push_str("<div>Terminal content not supported in WASM</div>");
            }
        }

        // Add slide notes if present
        if let Content::Text { text } = &slide.notes {
            if !text.is_empty() {
                content.push_str(&format!(
                    "<details><summary>Notes</summary><div>{}</div></details>",
                    escape_html(text)
                ));
            }
        }

        if content.is_empty() {
            content = "<p>Empty slide</p>".to_string();
        }

        self.app_element.set_inner_html(&content);
    }

    fn render_content_to_buffer(&self, content: &Content, buffer: &mut String) {
        match content {
            Content::Text { text } => {
                buffer.push_str("<div>");
                html_escape_to_buffer(text, buffer);
                buffer.push_str("</div>");
            }
            Content::Html { raw, .. } => {
                buffer.push_str("<div>");
                buffer.push_str(&sanitize_html(raw));
                buffer.push_str("</div>");
            }
            Content::IFrame { url } => {
                if self.is_safe_url(url) {
                    buffer.push_str("<iframe src=\"");
                    html_escape_to_buffer(url, buffer);
                    buffer.push_str("\" sandbox=\"allow-scripts allow-same-origin\" title=\"Embedded content\"></iframe>");
                } else {
                    buffer.push_str("<div>Unsafe URL blocked</div>");
                }
            }
            Content::Empty => {}
            Content::HBox { contents, .. } => {
                buffer.push_str("<div class=\"hbox\">");
                for item in contents {
                    self.render_content_to_buffer(item, buffer);
                }
                buffer.push_str("</div>");
            }
            Content::VBox { contents, .. } => {
                buffer.push_str("<div class=\"vbox\">");
                for item in contents {
                    self.render_content_to_buffer(item, buffer);
                }
                buffer.push_str("</div>");
            }
            Content::Term { .. } => {
                buffer.push_str("<div>Terminal content not supported in WASM</div>")
            }
        }
    }

    fn is_safe_url(&self, url: &str) -> bool {
        url.starts_with("https://")
            || url.starts_with("http://localhost")
            || url.starts_with("http://127.0.0.1")
    }

    fn update_slide_counter(&self) {
        if let Some(slide_id) = self.current_slide {
            // Find the position of the slide in our ordered cache
            let display_number = if let Some(cache) = &self.slides_cache {
                cache
                    .slides
                    .iter()
                    .position(|(id, _)| *id == slide_id)
                    .map(|pos| pos + 1)
                    .unwrap_or(1)
            } else {
                1 // Default fallback
            };

            let total = self
                .slides_cache
                .as_ref()
                .map(|c| c.total_count)
                .unwrap_or(0);
            let text = if total > 0 {
                format!("Slide: {display_number} / {total}")
            } else {
                format!("Slide: {display_number}")
            };
            self.slide_counter.set_text_content(Some(&text));
        } else {
            self.slide_counter.set_text_content(Some("Slide: - / -"));
        }
    }

    fn start_timer(&mut self) {
        self.stop_timer();

        // Update immediately
        self.update_duration_from_start_time();

        // Set up interval using gloo with safe weak reference
        let duration_display = self.duration_display.clone();
        let start_time = self.start_time;
        let timer = Interval::new(1000, move || {
            if let Some(start_time) = start_time {
                let elapsed_ms = window().performance().unwrap().now() - start_time;
                let elapsed_seconds = (elapsed_ms / 1000.0).floor() as u64;
                let formatted = format_duration(elapsed_seconds);
                duration_display.set_text_content(Some(&format!("Duration: {formatted}")));
            }
        });

        self.timer = Some(timer);
    }

    fn stop_timer(&mut self) {
        self.timer = None;
    }

    fn update_duration_from_start_time(&self) {
        if let Some(start_time) = self.start_time {
            if matches!(self.presentation_state, Some(State::Running { .. })) {
                let elapsed_ms = window().performance().unwrap().now() - start_time;
                let elapsed_seconds = (elapsed_ms / 1000.0).floor() as u64;
                self.update_duration_display(elapsed_seconds);
            }
        }
    }

    fn update_duration_display(&self, total_seconds: u64) {
        let formatted = format_duration(total_seconds);
        self.duration_display
            .set_text_content(Some(&format!("Duration: {formatted}")));
    }

    fn update_connection_status(&self, status: &str, class_name: &str) {
        self.connection_status.set_text_content(Some(status));
        self.connection_status.set_class_name(class_name);
    }

    fn show_error(&self, message: &str) {
        self.error_display.set_text_content(Some(message));
        self.error_display.set_class_name("error");
        let css_style = self.error_display.style();
        css_style.set_property("display", "block").unwrap_or(());

        // Auto-hide after 5 seconds using gloo
        let error_element = self.error_display.clone();
        let _ = Timeout::new(5000, move || {
            let css_style = error_element.style();
            css_style.set_property("display", "none").unwrap_or(());
        });
    }

    fn show_error_with_category(&self, error: &TobogganError) {
        let (message, class) = match error {
            TobogganError::WebSocketError(msg) => {
                (format!("Connection error: {msg}"), "error-connection")
            }
            TobogganError::ParseError(msg) => (format!("Data parsing error: {msg}"), "error-parse"),
            TobogganError::DomError(msg) => (format!("Interface error: {msg}"), "error-dom"),
            TobogganError::NetworkError(msg) => (format!("Network error: {msg}"), "error-network"),
            TobogganError::ConfigError(msg) => {
                (format!("Configuration error: {msg}"), "error-config")
            }
        };

        self.error_display.set_text_content(Some(&message));
        self.error_display.set_class_name(class);
        let css_style = self.error_display.style();
        css_style.set_property("display", "block").unwrap_or(());

        // Auto-hide after 8 seconds for categorized errors
        let error_element = self.error_display.clone();
        let _ = Timeout::new(8000, move || {
            let css_style = error_element.style();
            css_style.set_property("display", "none").unwrap_or(());
        });
    }
}

// Implementation for TobogganApp (public methods that need to work with the TobogganApp wrapper)
impl TobogganApp {
    fn setup_navigation_buttons(&self) -> Result<(), JsValue> {
        // Navigation commands mapping
        let commands = [
            ("first-btn", Command::First),
            ("prev-btn", Command::Previous),
            ("next-btn", Command::Next),
            ("last-btn", Command::Last),
            ("pause-btn", Command::Pause),
            ("resume-btn", Command::Resume),
        ];

        let state = self.state.borrow();
        let ws = state.websocket.clone();
        drop(state);

        for (button_id, command) in commands.iter() {
            if let Ok(element) = get_element_by_id(button_id) {
                let cmd = command.clone();
                let ws_clone = ws.clone();

                let listener = EventListener::new(&element, "click", move |_event| {
                    if let Some(ws_ref) = &ws_clone {
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            let _ = ws_ref.send_with_str(&json);
                        }
                    }
                });

                self.state.borrow_mut()._event_listeners.push(listener);
            }
        }

        Ok(())
    }

    fn setup_keyboard_handlers(&self) -> Result<(), JsValue> {
        let state = self.state.borrow();
        let ws = state.websocket.clone();
        drop(state);

        let listener = EventListener::new(&document(), "keydown", move |event| {
            if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
                let key = keyboard_event.key();
                let command = match key.as_str() {
                    "ArrowLeft" | "ArrowUp" => Some(Command::Previous),
                    "ArrowRight" | "ArrowDown" | " " => Some(Command::Next),
                    "Home" => Some(Command::First),
                    "End" => Some(Command::Last),
                    "p" | "P" => Some(Command::Pause),
                    "r" | "R" => Some(Command::Resume),
                    _ => None,
                };

                if let Some(cmd) = command {
                    keyboard_event.prevent_default();
                    if let Some(ws_ref) = &ws {
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            let _ = ws_ref.send_with_str(&json);
                        }
                    }
                }
            }
        });

        self.state.borrow_mut()._event_listeners.push(listener);
        Ok(())
    }
}

// Helper functions
fn get_element_by_id(id: &str) -> Result<HtmlElement, JsValue> {
    document()
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("Element with id '{id}' not found")))?
        .dyn_into::<HtmlElement>()
        .map_err(|_| JsValue::from_str(&format!("Element '{id}' is not an HtmlElement")))
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// Optimized HTML escaping that writes directly to a buffer
fn html_escape_to_buffer(text: &str, buffer: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => buffer.push_str("&amp;"),
            '<' => buffer.push_str("&lt;"),
            '>' => buffer.push_str("&gt;"),
            '"' => buffer.push_str("&quot;"),
            '\'' => buffer.push_str("&#39;"),
            _ => buffer.push(ch),
        }
    }
}

// Basic HTML sanitization for security
fn sanitize_html(html: &str) -> String {
    html.replace("<script", "&lt;script")
        .replace("</script", "&lt;/script")
        .replace("javascript:", "")
        .replace("vbscript:", "")
        .replace("data:", "")
        .replace("on", "&on") // Disable event handlers like onclick, onload, etc.
}

fn format_duration(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

// Export the main initialization function
#[wasm_bindgen]
pub fn init_app() -> Result<TobogganApp, JsValue> {
    TobogganApp::new()
}

// Export the initialization function with config
#[wasm_bindgen]
pub fn init_app_with_config(config: TobogganConfig) -> Result<TobogganApp, JsValue> {
    TobogganApp::new_with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regular unit tests for non-WASM specific functionality
    #[test]
    fn test_html_escaping() {
        let input = "<script>alert('xss')</script>";
        let escaped = escape_html(input);
        assert_eq!(escaped, "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    }

    #[test]
    fn test_html_sanitization() {
        let input = "<script src='evil.js'></script><p onclick='alert()'>text</p>";
        let sanitized = sanitize_html(input);
        assert!(sanitized.contains("&lt;script"));
        assert!(sanitized.contains("&onclick"));
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(3661), "01:01:01");
        assert_eq!(format_duration(0), "00:00:00");
        assert_eq!(format_duration(3600), "01:00:00");
    }

    #[test]
    fn test_config_creation() {
        let config = TobogganConfig::new();
        assert!(config.websocket_url.contains("ws://localhost:8080"));
        assert!(config.auto_retry);
    }
}

// WASM-specific tests (run with wasm-pack test)
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_wasm_app_creation() {
        // Test that we can create a TobogganConfig in WASM
        let config = TobogganConfig::new();
        assert!(config.websocket_url.contains("ws://localhost:8080"));
    }
}
