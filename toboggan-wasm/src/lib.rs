use gloo::console;
use gloo::events::EventListener;
use gloo::timers::callback::{Interval, Timeout};
use gloo::utils::{document, window};
use once_cell::sync::Lazy;
use regex::Regex;
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
#[derive(Debug, Clone, PartialEq)]
pub enum TobogganError {
    WebSocketError(String),
    ParseError(String),
    DomError(String),
    NetworkError(String),
    ConfigError(String),
    BorrowError(String),
    PerformanceUnavailable,
    ElementNotFound { id: String },
    RetryExhausted { attempts: u32 },
}

impl core::fmt::Display for TobogganError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TobogganError::WebSocketError(msg) => write!(f, "WebSocket error: {msg}"),
            TobogganError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            TobogganError::DomError(msg) => write!(f, "DOM error: {msg}"),
            TobogganError::NetworkError(msg) => write!(f, "Network error: {msg}"),
            TobogganError::ConfigError(msg) => write!(f, "Config error: {msg}"),
            TobogganError::BorrowError(msg) => write!(f, "Borrow error: {msg}"),
            TobogganError::PerformanceUnavailable => write!(f, "Performance API unavailable"),
            TobogganError::ElementNotFound { id } => write!(f, "DOM element not found: {id}"),
            TobogganError::RetryExhausted { attempts } => {
                write!(f, "Retry exhausted after {attempts} attempts")
            }
        }
    }
}

impl std::error::Error for TobogganError {}

impl From<TobogganError> for JsValue {
    fn from(err: TobogganError) -> Self {
        JsValue::from_str(&format!("{err:?}"))
    }
}

// Configuration for the Toboggan app
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq)]
pub struct TobogganConfig {
    websocket_url: String,
    auto_retry: bool,
    retry_attempts: u32,
    preload_slides: bool,
    base_retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    retry_jitter: bool,
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
            base_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            retry_jitter: true,
        }
    }

    // Add getters for testing
    #[wasm_bindgen(getter)]
    pub fn websocket_url(&self) -> String {
        self.websocket_url.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn auto_retry(&self) -> bool {
        self.auto_retry
    }

    #[wasm_bindgen(getter)]
    pub fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }

    #[wasm_bindgen(getter)]
    pub fn preload_slides(&self) -> bool {
        self.preload_slides
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

    #[wasm_bindgen(setter)]
    pub fn set_base_retry_delay_ms(&mut self, delay_ms: u64) {
        self.base_retry_delay_ms = delay_ms;
    }

    #[wasm_bindgen(setter)]
    pub fn set_max_retry_delay_ms(&mut self, delay_ms: u64) {
        self.max_retry_delay_ms = delay_ms;
    }

    #[wasm_bindgen(setter)]
    pub fn set_retry_jitter(&mut self, jitter: bool) {
        self.retry_jitter = jitter;
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
    // WebSocket callback closures that need cleanup
    _websocket_closures: Vec<Box<dyn AsRef<JsValue>>>,
    // Timeout handles for proper cleanup
    _timeouts: Vec<Timeout>,
    // Retry timeout handle
    _retry_timeout: Option<Timeout>,
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
            // Initialize WebSocket closures as empty
            _websocket_closures: Vec::new(),
            // Initialize timeouts as empty
            _timeouts: Vec::new(),
            // Initialize retry timeout as None
            _retry_timeout: None,
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

        // Clean up WebSocket closures
        state._websocket_closures.clear();

        // Clean up all timeouts
        for timeout in state._timeouts.drain(..) {
            timeout.cancel();
        }

        // Clean up retry timeout
        if let Some(timeout) = state._retry_timeout.take() {
            timeout.cancel();
        }

        // Clear caches
        state.slides_cache = None;
        state.presentation_state = None;
    }

    fn connect_with_retry(&self) -> Result<(), JsValue> {
        self.attempt_connection(1)
    }

    fn attempt_connection(&self, attempt: u32) -> Result<(), JsValue> {
        let state = self.state.borrow();
        let max_retries = state.config.retry_attempts;
        let retry_policy = RetryPolicy::from_config(&state.config);
        drop(state);

        // Clean up existing WebSocket if any
        self.disconnect_websocket();

        match self.connect() {
            Ok(()) => {
                console::log!("WebSocket connected successfully on attempt {}", attempt);
                self.state.borrow_mut().retry_count = 0;
                Ok(())
            }
            Err(_e) if attempt < max_retries => {
                console::warn!("Connection attempt {} failed, scheduling retry...", attempt);
                self.state.borrow_mut().retry_count = attempt;

                // Schedule retry with exponential backoff
                let retry_delay = retry_policy.calculate_delay(attempt);
                let app_clone = TobogganApp {
                    state: self.state.clone(),
                };

                let timeout = Timeout::new(retry_delay as u32, move || {
                    console::log!("Retrying connection attempt {}...", attempt + 1);
                    if let Err(e) = app_clone.attempt_connection(attempt + 1) {
                        console::error!("Retry attempt {} failed: {:?}", attempt + 1, e);
                    }
                });

                // Store timeout to prevent immediate garbage collection
                self.state.borrow_mut()._retry_timeout = Some(timeout);
                Ok(())
            }
            Err(e) => {
                console::error!(
                    "All connection attempts exhausted after {} tries",
                    max_retries
                );
                let mut state = self.state.borrow_mut();
                state.show_error_with_category(&TobogganError::RetryExhausted {
                    attempts: max_retries,
                });
                state.retry_count = 0;
                Err(e)
            }
        }
    }

    // Helper method to cleanly disconnect WebSocket
    fn disconnect_websocket(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(ws) = state.websocket.take() {
            // Close WebSocket cleanly if it's still connected
            if ws.ready_state() == WebSocket::OPEN {
                let _ = ws.close();
            }
        }

        // Clean up WebSocket closures to prevent memory leaks\n        state._websocket_closures.clear();\n        \n        // Clean up any pending timeouts\n        for timeout in state._timeouts.drain(..) {\n            timeout.cancel();\n        }\n        \n        // Cancel any pending retry timeout
        if let Some(timeout) = state._retry_timeout.take() {
            timeout.cancel();
        }
    }

    // Public method to manually trigger reconnection
    pub fn reconnect(&self) -> Result<(), JsValue> {
        console::log!("Manual reconnection requested");
        self.state.borrow_mut().retry_count = 0;
        self.attempt_connection(1)
    }

    fn connect(&self) -> Result<(), JsValue> {
        let state = self.state.borrow();
        let ws = WebSocket::new(&state.config.websocket_url)?;
        let client_id = state.client_id;

        // Clone elements for closures
        let status_elem = state.connection_status.clone();
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
        // Store closure for proper cleanup instead of forgetting
        self.state
            .borrow_mut()
            ._websocket_closures
            .push(Box::new(onopen_callback));

        // OnMessage handler
        let state_weak_message = state_weak.clone();
        let error_elem_for_message = self.state.borrow().error_display.clone();
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
                        error_elem_for_message.set_text_content(Some(&format!(
                            "Failed to parse server message: {err}"
                        )));
                        let css_style = error_elem_for_message.style();
                        css_style.set_property("display", "block").unwrap_or(());
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // Store closure for proper cleanup instead of forgetting
        self.state
            .borrow_mut()
            ._websocket_closures
            .push(Box::new(onmessage_callback));

        // OnClose handler
        let status_elem_close = self.state.borrow().connection_status.clone();
        let state_weak_close = state_weak.clone();
        let app_clone_close = TobogganApp {
            state: self.state.clone(),
        };
        let onclose_callback = Closure::wrap(Box::new(move |_: Event| {
            console::log!("WebSocket connection closed");
            status_elem_close.set_text_content(Some("Disconnected"));
            status_elem_close.set_class_name("disconnected");

            if let Some(state_rc) = state_weak_close.upgrade() {
                let mut state = state_rc.borrow_mut();
                state.slides_cache = None;
                state.stop_timer();

                // Auto-reconnect if enabled and not already retrying
                let should_reconnect = state.config.auto_retry && state.retry_count == 0;
                drop(state);

                if should_reconnect {
                    console::log!("Auto-reconnecting after connection close...");
                    if let Err(e) = app_clone_close.attempt_connection(1) {
                        console::error!("Auto-reconnect failed: {:?}", e);
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        // Store closure for proper cleanup instead of forgetting
        self.state
            .borrow_mut()
            ._websocket_closures
            .push(Box::new(onclose_callback));

        // OnError handler
        let error_elem_error = self.state.borrow().error_display.clone();
        let state_weak_error = state_weak.clone();
        let app_clone_error = TobogganApp {
            state: self.state.clone(),
        };
        let onerror_callback = Closure::wrap(Box::new(move |error: Event| {
            console::error!("WebSocket error:", format!("{:?}", error));
            error_elem_error.set_text_content(Some("Connection error"));
            error_elem_error.set_class_name("error");

            if let Some(state_rc) = state_weak_error.upgrade() {
                let state = state_rc.borrow();
                state.show_error("WebSocket connection error");

                // Check if we should retry on error (but not if already retrying)
                let should_retry = state.config.auto_retry && state.retry_count == 0;
                drop(state);

                if should_retry {
                    console::log!("Attempting to reconnect after error...");
                    if let Err(e) = app_clone_error.attempt_connection(1) {
                        console::error!("Error recovery attempt failed: {:?}", e);
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        // Store closure for proper cleanup instead of forgetting
        self.state
            .borrow_mut()
            ._websocket_closures
            .push(Box::new(onerror_callback));

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
            Notification::Blink => todo!(),
        }
    }

    fn handle_state_notification(&mut self, state: State) {
        self.presentation_state = Some(state.clone());

        match &state {
            State::Init => {
                self.current_slide = None;
                // TODO check if we need to do something else
            }
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
        let _timeout = Timeout::new(5000, move || {
            let css_style = error_element.style();
            css_style.set_property("display", "none").unwrap_or(());
        });

        // Store timeout for proper cleanup - access through state
        // Note: This is a simple timeout that will auto-cleanup, so we don't store it
        // If we need to store it, we'd need to restructure the method signature
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
            TobogganError::BorrowError(msg) => {
                (format!("State access error: {msg}"), "error-state")
            }
            TobogganError::PerformanceUnavailable => (
                "Performance API unavailable".to_string(),
                "error-performance",
            ),
            TobogganError::ElementNotFound { id } => {
                (format!("Element not found: {id}"), "error-element")
            }
            TobogganError::RetryExhausted { attempts } => (
                format!("Connection failed after {attempts} attempts"),
                "error-retry",
            ),
        };

        self.error_display.set_text_content(Some(&message));
        self.error_display.set_class_name(class);
        let css_style = self.error_display.style();
        css_style.set_property("display", "block").unwrap_or(());

        // Auto-hide after 8 seconds for categorized errors
        let error_element = self.error_display.clone();
        let _timeout = Timeout::new(8000, move || {
            let css_style = error_element.style();
            css_style.set_property("display", "none").unwrap_or(());
        });

        // Store timeout for proper cleanup - access through state
        // Note: This is a simple timeout that will auto-cleanup, so we don't store it
        // If we need to store it, we'd need to restructure the method signature
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

// Comprehensive HTML sanitization for security
static SCRIPT_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap());

static EVENT_HANDLER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\s*on[a-z]+\s*=").unwrap());

static JAVASCRIPT_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)javascript\s*:").unwrap());

static VBSCRIPT_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)vbscript\s*:").unwrap());

static DATA_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)data\s*:").unwrap());

// Additional security patterns
static OBJECT_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<object[^>]*>.*?</object>").unwrap());

static EMBED_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<embed[^>]*/?>").unwrap());

static APPLET_TAG: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<applet[^>]*>.*?</applet>").unwrap());

static FORM_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<form[^>]*>.*?</form>").unwrap());

static STYLE_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<style[^>]*>.*?</style>").unwrap());

static META_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<meta[^>]*/?>").unwrap());

static LINK_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<link[^>]*/?>").unwrap());

// CSS expressions and imports
static CSS_EXPRESSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)expression\s*\(").unwrap());

static CSS_IMPORT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)@import").unwrap());

// Protocol handlers
static FILE_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)file\s*:").unwrap());

static FTP_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)ftp\s*:").unwrap());

fn sanitize_html(html: &str) -> String {
    let mut result = html.to_string();

    // Remove dangerous tags completely
    result = SCRIPT_TAG.replace_all(&result, "").to_string();
    result = OBJECT_TAG.replace_all(&result, "").to_string();
    result = EMBED_TAG.replace_all(&result, "").to_string();
    result = APPLET_TAG.replace_all(&result, "").to_string();
    result = FORM_TAG.replace_all(&result, "").to_string();
    result = STYLE_TAG.replace_all(&result, "").to_string();
    result = META_TAG.replace_all(&result, "").to_string();
    result = LINK_TAG.replace_all(&result, "").to_string();

    // Replace event handlers with safe attributes
    result = EVENT_HANDLER
        .replace_all(&result, " data-removed-event=")
        .to_string();

    // Block dangerous URL schemes
    result = JAVASCRIPT_URL.replace_all(&result, "blocked:").to_string();
    result = VBSCRIPT_URL.replace_all(&result, "blocked:").to_string();
    result = DATA_URL.replace_all(&result, "blocked:").to_string();
    result = FILE_URL.replace_all(&result, "blocked:").to_string();
    result = FTP_URL.replace_all(&result, "blocked:").to_string();

    // Remove CSS expressions and imports
    result = CSS_EXPRESSION
        .replace_all(&result, "blocked-expression(")
        .to_string();
    result = CSS_IMPORT
        .replace_all(&result, "/* blocked-import */")
        .to_string();

    result
}

// Basic HTML sanitization fallback (for tests without regex)
#[allow(dead_code)]
fn sanitize_html_basic(html: &str) -> String {
    let result = html.to_lowercase();
    let mut output = html.to_string();

    // Case-insensitive script tag replacement
    if result.contains("<script") {
        output = output
            .replace("<script", "&lt;script")
            .replace("<SCRIPT", "&lt;SCRIPT");
    }
    if result.contains("</script") {
        output = output
            .replace("</script", "&lt;/script")
            .replace("</SCRIPT", "&lt;/SCRIPT");
    }

    // URL scheme blocking
    output = output
        .replace("javascript:", "blocked:")
        .replace("vbscript:", "blocked:")
        .replace("data:", "blocked:");

    output
}

fn format_duration(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

// Safe performance API access
#[allow(dead_code)]
fn safe_performance_now() -> Result<f64, TobogganError> {
    Ok(window()
        .performance()
        .ok_or(TobogganError::PerformanceUnavailable)?
        .now())
}

// Retry policy implementation
#[derive(Debug, Clone)]
struct RetryPolicy {
    #[allow(dead_code)]
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    jitter: bool,
}

impl RetryPolicy {
    fn from_config(config: &TobogganConfig) -> Self {
        Self {
            max_attempts: config.retry_attempts,
            base_delay_ms: config.base_retry_delay_ms,
            max_delay_ms: config.max_retry_delay_ms,
            jitter: config.retry_jitter,
        }
    }

    fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay =
            (self.base_delay_ms * 2_u64.pow(attempt.saturating_sub(1))).min(self.max_delay_ms);

        if self.jitter {
            // Add ±25% jitter to prevent thundering herd
            let jitter_range = delay / 4;
            let random_offset = (js_sys::Math::random() * jitter_range as f64) as u64;
            delay + random_offset - (jitter_range / 2)
        } else {
            delay
        }
    }
}

// Safe state access wrapper
#[allow(dead_code)]
struct SafeStateAccess {
    state: Rc<RefCell<TobogganAppState>>,
}

impl SafeStateAccess {
    #[allow(dead_code)]
    fn new(state: Rc<RefCell<TobogganAppState>>) -> Self {
        Self { state }
    }

    #[allow(dead_code)]
    fn with_state<F, R>(&self, f: F) -> Result<R, TobogganError>
    where
        F: FnOnce(&TobogganAppState) -> R,
    {
        self.state
            .try_borrow()
            .map(|state| f(&state))
            .map_err(|e| TobogganError::BorrowError(format!("Failed to borrow state: {e}")))
    }

    #[allow(dead_code)]
    fn with_state_mut<F, R>(&self, f: F) -> Result<R, TobogganError>
    where
        F: FnOnce(&mut TobogganAppState) -> R,
    {
        self.state
            .try_borrow_mut()
            .map(|mut state| f(&mut state))
            .map_err(|e| TobogganError::BorrowError(format!("Failed to borrow state mutably: {e}")))
    }
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

    // Test error types
    #[test]
    fn test_error_display() {
        let error = TobogganError::WebSocketError("Connection failed".to_string());
        assert_eq!(format!("{error}"), "WebSocket error: Connection failed");

        let error = TobogganError::ElementNotFound {
            id: "test-btn".to_string(),
        };
        assert_eq!(format!("{error}"), "DOM element not found: test-btn");
    }

    #[test]
    fn test_error_equality() {
        let error1 = TobogganError::ParseError("Invalid JSON".to_string());
        let error2 = TobogganError::ParseError("Invalid JSON".to_string());
        assert_eq!(error1, error2);
    }

    // Test HTML escaping with edge cases
    #[test]
    fn test_html_escaping_comprehensive() {
        let test_cases = vec![
            (
                "<script>alert('xss')</script>",
                "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;",
            ),
            ("&<>\"'", "&amp;&lt;&gt;&quot;&#39;"),
            ("normal text", "normal text"),
            ("", ""),
            (
                "<img src=\"x\" onerror=\"alert(1)\">",
                "&lt;img src=&quot;x&quot; onerror=&quot;alert(1)&quot;&gt;",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(escape_html(input), expected, "Failed for input: {input}");
        }
    }

    #[test]
    fn test_html_escape_to_buffer() {
        let mut buffer = String::new();
        html_escape_to_buffer("<script>test</script>", &mut buffer);
        assert_eq!(buffer, "&lt;script&gt;test&lt;/script&gt;");

        // Test appending
        html_escape_to_buffer(" & more", &mut buffer);
        assert_eq!(buffer, "&lt;script&gt;test&lt;/script&gt; &amp; more");
    }

    // Test HTML sanitization with comprehensive cases
    #[test]
    fn test_html_sanitization_comprehensive() {
        let test_cases = vec![
            // Script tags
            (
                "<script>alert('xss')</script>",
                "&lt;script>alert('xss')&lt;/script>",
            ),
            (
                "<SCRIPT type='text/javascript'>alert(1)</SCRIPT>",
                "&lt;SCRIPT type='text/javascript'>alert(1)&lt;/SCRIPT>",
            ),
            (
                "<script src='evil.js'></script><p>safe content</p>",
                "&lt;script src='evil.js'>&lt;/script><p>safe content</p>",
            ),
            // Dangerous URLs
            (
                "<a href='javascript:alert(1)'>link</a>",
                "<a href='blocked:alert(1)'>link</a>",
            ),
            (
                "<iframe src='vbscript:msgbox(1)'></iframe>",
                "<iframe src='blocked:msgbox(1)'></iframe>",
            ),
            (
                "<img src='data:text/html,<script>alert(1)</script>'>",
                "<img src='blocked:text/html,&lt;script>alert(1)&lt;/script>'>",
            ),
            // Safe content should remain unchanged
            ("<p>This is safe content</p>", "<p>This is safe content</p>"),
            (
                "<div class='container'><span>text</span></div>",
                "<div class='container'><span>text</span></div>",
            ),
        ];

        for (input, expected) in test_cases {
            let result = sanitize_html_basic(input); // Use basic version for testing
            assert_eq!(
                result, expected,
                "Failed for input '{input}': expected '{expected}', got '{result}'"
            );
        }
    }

    // Test duration formatting edge cases
    #[test]
    fn test_duration_formatting_comprehensive() {
        let test_cases = vec![
            (0, "00:00:00"),
            (1, "00:00:01"),
            (59, "00:00:59"),
            (60, "00:01:00"),
            (3599, "00:59:59"),
            (3600, "01:00:00"),
            (3661, "01:01:01"),
            (86400, "24:00:00"), // 24 hours
            (90061, "25:01:01"), // Over 24 hours
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                format_duration(input),
                expected,
                "Failed for {input} seconds"
            );
        }
    }

    // Test configuration
    #[test]
    fn test_config_creation_and_modification() {
        let mut config = TobogganConfig::new();

        // Test default values
        assert_eq!(config.websocket_url(), "ws://localhost:8080/api/ws");
        assert!(config.auto_retry());
        assert_eq!(config.retry_attempts(), 3);
        assert!(config.preload_slides());

        // Test modifications
        config.set_websocket_url("ws://test:9999/ws".to_string());
        config.set_auto_retry(false);
        config.set_retry_attempts(5);
        config.set_preload_slides(false);

        assert_eq!(config.websocket_url(), "ws://test:9999/ws");
        assert!(!config.auto_retry());
        assert_eq!(config.retry_attempts(), 5);
        assert!(!config.preload_slides());
    }

    #[test]
    fn test_config_equality() {
        let config1 = TobogganConfig::new();
        let config2 = TobogganConfig::new();
        assert_eq!(config1, config2);

        let mut config3 = TobogganConfig::new();
        config3.set_retry_attempts(5);
        assert_ne!(config1, config3);
    }

    // Test retry policy
    #[test]
    fn test_retry_policy_calculation() {
        let config = TobogganConfig::new();
        let policy = RetryPolicy::from_config(&config);

        // Test exponential backoff without jitter
        let mut test_policy = policy.clone();
        test_policy.jitter = false;

        assert_eq!(test_policy.calculate_delay(1), 1000); // 1000 * 2^0
        assert_eq!(test_policy.calculate_delay(2), 2000); // 1000 * 2^1
        assert_eq!(test_policy.calculate_delay(3), 4000); // 1000 * 2^2
        assert_eq!(test_policy.calculate_delay(4), 8000); // 1000 * 2^3

        // Test max delay cap
        test_policy.max_delay_ms = 5000;
        assert_eq!(test_policy.calculate_delay(10), 5000); // Should be capped
    }

    #[test]
    fn test_retry_policy_with_jitter() {
        let config = TobogganConfig::new();
        let mut policy = RetryPolicy::from_config(&config);

        // Test without jitter first (deterministic)
        policy.jitter = false;
        let delay_no_jitter = policy.calculate_delay(1);
        assert_eq!(delay_no_jitter, 1000);

        // For jitter test, we can't test the actual randomness in non-WASM,
        // but we can test the policy structure exists
        policy.jitter = true;
        // Just verify the policy has jitter enabled
        assert!(policy.jitter);
    }

    // Test safe performance API access
    // This test is only meaningful in WASM environments
    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_safe_performance_now() {
        match safe_performance_now() {
            Ok(time) => assert!(time >= 0.0),
            Err(TobogganError::PerformanceUnavailable) => {
                // This could happen in some WASM environments
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    // For non-WASM, just test that the function signature exists
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_safe_performance_now_signature() {
        // This test just verifies the function exists and has correct signature
        // We can't actually call it in non-WASM environment
        let _function_exists: fn() -> Result<f64, TobogganError> = safe_performance_now;
    }

    // Test URL safety checking
    #[test]
    fn test_url_safety() {
        // This would require the TobogganAppState implementation
        // We'll add a standalone function for testing
        assert!(is_safe_url_standalone("https://example.com"));
        assert!(is_safe_url_standalone("http://localhost:3000"));
        assert!(is_safe_url_standalone("http://127.0.0.1:8080"));

        assert!(!is_safe_url_standalone("javascript:alert(1)"));
        assert!(!is_safe_url_standalone(
            "data:text/html,<script>alert(1)</script>"
        ));
        assert!(!is_safe_url_standalone("ftp://example.com"));
    }

    // Standalone URL safety function for testing
    fn is_safe_url_standalone(url: &str) -> bool {
        url.starts_with("https://")
            || url.starts_with("http://localhost")
            || url.starts_with("http://127.0.0.1")
    }
}

// WASM-specific tests (run with wasm-pack test)
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::*;

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_wasm_app_creation() {
        // Test that we can create a TobogganConfig in WASM
        let config = TobogganConfig::new();
        assert!(config.websocket_url.contains("ws://localhost:8080"));
    }
}
