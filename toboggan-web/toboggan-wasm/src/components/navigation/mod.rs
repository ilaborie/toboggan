use futures::channel::mpsc::UnboundedSender;
use gloo::events::EventListener;
use gloo::timers::callback::Interval;
use toboggan_core::{Command, Duration, State, TalkResponse};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlButtonElement, HtmlElement, HtmlProgressElement, ShadowRoot};

use crate::components::WasmElement;
use crate::{
    ConnectionStatus, StateClassMapper, create_and_append_element, create_button,
    create_shadow_root_with_style, dom_try, dom_try_or_return, format_duration, render_content,
    unwrap_or_return,
};

#[derive(Debug, Default)]
pub struct TobogganNavigationElement {
    parent: Option<HtmlElement>,
    root: Option<ShadowRoot>,

    container: Option<Element>,

    progress_el: Option<HtmlProgressElement>,
    talk_el: Option<Element>,
    connection_status_el: Option<Element>,
    slide_info_el: Option<Element>,
    duration_el: Option<Element>,

    // Navigation buttons
    home_btn: Option<HtmlButtonElement>,
    prev_btn: Option<HtmlButtonElement>,
    next_btn: Option<HtmlButtonElement>,
    last_btn: Option<HtmlButtonElement>,
    pause_btn: Option<HtmlButtonElement>,
    resume_btn: Option<HtmlButtonElement>,

    state: Option<State>,
    talk: Option<TalkResponse>,
    connection_status: Option<ConnectionStatus>,
    slide_current: Option<usize>,
    slide_count: Option<usize>,
    duration: Option<Duration>,
    current_duration: Option<Duration>,

    // Timer for auto-incrementing duration
    duration_timer: Option<Interval>,

    // Command sender for navigation actions
    tx_cmd: Option<UnboundedSender<Command>>,

    listeners: Vec<EventListener>,
}

impl TobogganNavigationElement {
    pub fn set_command_sender(&mut self, tx_cmd: UnboundedSender<Command>) {
        self.tx_cmd = Some(tx_cmd);
        self.setup_button_handlers();
    }

    pub fn set_state(&mut self, state: Option<State>) {
        self.set_slide_current(state.as_ref().and_then(State::current));
        self.set_duration(state.as_ref().map(State::calculate_total_duration));
        self.state = state;
        self.render_state();
    }

    fn render_state(&mut self) {
        let container = unwrap_or_return!(&self.container);

        let mut classes = vec![];

        // Add state class
        if let Some(state) = &self.state {
            classes.push(state.to_css_class());
        } else {
            classes.push("none");
        }

        // Add position classes for CSS-based button visibility
        let current_slide = self
            .slide_current
            .and_then(|id| id.to_string().parse::<usize>().ok())
            .unwrap_or(0);
        let slide_count = self.slide_count.unwrap_or(1);

        if current_slide == 0 {
            classes.push("at-first");
        }
        if current_slide >= slide_count.saturating_sub(1) {
            classes.push("at-last");
        }

        container.set_class_name(&classes.join(" "));
        self.manage_duration_timer();
    }

    pub fn set_talk(&mut self, talk: Option<TalkResponse>) {
        self.talk = talk;
        self.render_talk();
    }

    fn render_talk(&mut self) {
        let talk_el = unwrap_or_return!(&self.talk_el);
        if let Some(talk) = &self.talk {
            let content = render_content(&talk.title, Some("h1"));
            talk_el.set_inner_html(&content);
        }
    }

    pub fn set_connection_status(&mut self, connection_status: Option<ConnectionStatus>) {
        self.connection_status = connection_status;
        self.render_connection_status();
    }

    fn render_connection_status(&mut self) {
        let connection_status_el = unwrap_or_return!(&self.connection_status_el);

        if let Some(status) = &self.connection_status {
            let class_name = status.to_css_class();
            connection_status_el.set_class_name(&format!("connection {class_name}"));

            // Set text content based on status type
            match status {
                ConnectionStatus::Connected => {
                    // Connected: only icon, no text
                    connection_status_el.set_text_content(None);
                }
                ConnectionStatus::Connecting => {
                    // Connecting: icon + static text (handled by CSS ::after)
                    connection_status_el.set_text_content(None);
                }
                ConnectionStatus::Closed => {
                    // Closed: icon + static text (handled by CSS ::after)
                    connection_status_el.set_text_content(None);
                }
                ConnectionStatus::Reconnecting { .. } | ConnectionStatus::Error { .. } => {
                    // Reconnecting/Error: icon + dynamic text (from Rust)
                    connection_status_el.set_text_content(Some(&status.to_string()));
                }
            }
        } else {
            connection_status_el.set_class_name("connection none");
            connection_status_el.set_text_content(None);
        }
    }

    pub fn set_slide_current(&mut self, slide_current: Option<usize>) {
        self.slide_current = slide_current;
        self.render_slide_info();
        self.update_progress_bar();
        self.render_state(); // Update CSS classes for button visibility
    }

    pub fn set_slide_count(&mut self, slide_count: Option<usize>) {
        self.slide_count = slide_count;
        self.render_slide_info();
        self.update_progress_bar();
        self.render_state(); // Update CSS classes for button visibility
    }

    fn render_slide_info(&mut self) {
        let slide_info_el = unwrap_or_return!(&self.slide_info_el);

        // Use data attributes for CSS-based formatting
        if let Some(current) = self.slide_current {
            let current_display = current + 1; // Convert to 1-based display
            dom_try!(
                slide_info_el.set_attribute("data-current", &current_display.to_string()),
                "set data-current attribute"
            );
        } else {
            dom_try!(
                slide_info_el.remove_attribute("data-current"),
                "remove data-current attribute"
            );
        }

        if let Some(total) = self.slide_count {
            dom_try!(
                slide_info_el.set_attribute("data-total", &total.to_string()),
                "set data-total attribute"
            );
        } else {
            dom_try!(
                slide_info_el.remove_attribute("data-total"),
                "remove data-total attribute"
            );
        }

        // Clear text content - CSS will handle display via pseudo-elements
        slide_info_el.set_text_content(Some(""));
    }

    fn update_progress_bar(&mut self) {
        let progress_el = unwrap_or_return!(&self.progress_el);

        // Set max value (0-indexed)
        #[allow(clippy::cast_precision_loss)]
        let max_value = self
            .slide_count
            .and_then(|count| (count > 0).then(|| (count - 1) as f64))
            .unwrap_or(1.0);
        progress_el.set_max(max_value);

        // Set current value
        #[allow(clippy::cast_precision_loss)]
        let value = self.slide_current.map_or(0.0, |index| index as f64);
        progress_el.set_value(value);
    }

    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.duration = duration;
        self.current_duration = duration;
        self.render_duration();

        // Restart timer if we're in running state
        if matches!(self.state, Some(State::Running { .. })) {
            self.start_duration_timer();
        }
    }

    fn manage_duration_timer(&mut self) {
        if matches!(self.state, Some(State::Running { .. })) {
            self.start_duration_timer();
        } else {
            self.stop_duration_timer();
        }
    }

    fn start_duration_timer(&mut self) {
        self.stop_duration_timer();

        if let Some(duration_el) = &self.duration_el {
            let duration_el_clone = duration_el.clone();
            let mut current_seconds = self
                .current_duration
                .map_or(0, |duration| core::time::Duration::from(duration).as_secs());

            let timer = Interval::new(1000, move || {
                current_seconds += 1;
                let time_str = format_duration(current_seconds);
                duration_el_clone.set_text_content(Some(&time_str));
            });

            self.duration_timer = Some(timer);
        }
    }

    fn stop_duration_timer(&mut self) {
        if let Some(timer) = self.duration_timer.take() {
            timer.cancel();
        }
    }

    fn render_duration(&mut self) {
        let duration_el = unwrap_or_return!(&self.duration_el);

        // Set duration text or leave empty (CSS will show default via ::before)
        if let Some(duration) = self.current_duration.or(self.duration) {
            duration_el.set_text_content(Some(&duration.to_string()));
        } else {
            duration_el.set_text_content(Some(""));
        }
    }

    fn setup_button_handlers(&mut self) {
        let tx_cmd = unwrap_or_return!(&self.tx_cmd);

        self.listeners.clear();

        // Helper macro to add button handler
        macro_rules! add_handler {
            ($button:expr, $command:expr) => {
                if let Some(btn) = $button {
                    let tx_cmd_clone = tx_cmd.clone();
                    let listener = EventListener::new(btn, "click", move |_| {
                        let _ = tx_cmd_clone.unbounded_send($command);
                    });
                    self.listeners.push(listener);
                }
            };
        }

        add_handler!(&self.home_btn, Command::First);
        add_handler!(&self.prev_btn, Command::Previous);
        add_handler!(&self.next_btn, Command::Next);
        add_handler!(&self.last_btn, Command::Last);
        add_handler!(&self.pause_btn, Command::Pause);
        add_handler!(&self.resume_btn, Command::Resume);
    }
}

impl WasmElement for TobogganNavigationElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try_or_return!(
            create_shadow_root_with_style(host, include_str!("./style.css")),
            "create shadow root"
        );

        // Create main nav container
        let nav_el: Element = dom_try_or_return!(
            create_and_append_element(&root, "nav"),
            "create nav element"
        );

        // Create progress bar
        let progress_el: HtmlProgressElement = dom_try_or_return!(
            create_and_append_element(&nav_el, "progress"),
            "create progress element"
        );
        progress_el.set_max(1.0);
        progress_el.set_value(0.0);

        // Create title element
        let title_el: Element = dom_try_or_return!(
            create_and_append_element(&nav_el, "h1"),
            "create title element"
        );

        // Create buttons area
        let buttons_el: Element = dom_try_or_return!(
            create_and_append_element(&nav_el, "div"),
            "create buttons container"
        );
        buttons_el.set_class_name("buttons");

        // Create navigation buttons with CSS classes (icons handled by CSS)
        let button_configs = [
            ("Go to first slide", "home-btn"),
            ("Previous slide", "prev-btn"),
            ("Pause presentation", "pause-btn"),
            ("Resume presentation", "resume-btn"),
            ("Next slide", "next-btn"),
            ("Go to last slide", "last-btn"),
        ];

        let buttons = button_configs
            .iter()
            .map(|(title, css_class)| {
                let btn = create_button("", title).unwrap_or_else(|_| {
                    // Fallback if create_button fails
                    create_and_append_element::<HtmlButtonElement>(&buttons_el, "button")
                        .unwrap_throw()
                });
                btn.set_class_name(css_class);
                dom_try!(buttons_el.append_child(&btn), "append button");
                btn
            })
            .collect::<Vec<_>>();

        // Create status area
        let status_el: Element = dom_try_or_return!(
            create_and_append_element(&nav_el, "div"),
            "create status element"
        );
        status_el.set_class_name("status");

        let connection_el: Element = dom_try_or_return!(
            create_and_append_element(&status_el, "span"),
            "create connection element"
        );
        connection_el.set_class_name("connection");

        let slide_info_el: Element = dom_try_or_return!(
            create_and_append_element(&status_el, "span"),
            "create slide info element"
        );
        slide_info_el.set_class_name("slide-info");

        let duration_el: Element = dom_try_or_return!(
            create_and_append_element(&status_el, "span"),
            "create duration element"
        );
        duration_el.set_class_name("duration");

        // Store references
        self.root = Some(root);
        self.parent = Some(host.clone());
        self.container = Some(nav_el);
        self.progress_el = Some(progress_el);
        self.talk_el = Some(title_el);
        self.connection_status_el = Some(connection_el);
        self.slide_info_el = Some(slide_info_el);
        self.duration_el = Some(duration_el);

        // Store button references (assuming order matches config)
        #[allow(clippy::indexing_slicing)]
        if buttons.len() >= 6 {
            self.home_btn = Some(buttons[0].clone());
            self.prev_btn = Some(buttons[1].clone());
            self.pause_btn = Some(buttons[2].clone());
            self.resume_btn = Some(buttons[3].clone());
            self.next_btn = Some(buttons[4].clone());
            self.last_btn = Some(buttons[5].clone());
        }

        // Initial render
        self.render_talk();
        self.render_state();
        self.render_connection_status();
        self.render_slide_info();
        self.render_duration();

        // Set up event handlers if command sender is already available
        if self.tx_cmd.is_some() {
            self.setup_button_handlers();
        }
    }
}

impl Drop for TobogganNavigationElement {
    fn drop(&mut self) {
        self.listeners.clear();
        self.stop_duration_timer();
    }
}
