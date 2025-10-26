use futures::channel::mpsc::UnboundedSender;
use gloo::events::EventListener;
use gloo::timers::callback::Interval;
use toboggan_core::{Command, Duration, State, TalkResponse};
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlButtonElement, HtmlElement, HtmlProgressElement};

use crate::components::WasmElement;
use crate::{
    ConnectionStatus, StateClassMapper, create_and_append_element, create_button,
    create_shadow_root_with_style, dom_try, format_duration,
};

const CSS: &str = include_str!("style.css");

#[derive(Debug, Default)]
pub struct TobogganNavigationElement {
    container: Option<Element>,

    progress_el: Option<HtmlProgressElement>,
    title_el: Option<Element>,
    connection_status_el: Option<Element>,
    slide_info_el: Option<Element>,
    duration_el: Option<Element>,

    buttons: Vec<HtmlButtonElement>,

    state: Option<State>,
    talk: Option<TalkResponse>,
    connection_status: Option<ConnectionStatus>,
    slide_current: Option<usize>,
    slide_count: Option<usize>,
    duration: Option<Duration>,

    duration_timer: Option<Interval>,
    tx_cmd: Option<UnboundedSender<Command>>,
    listeners: Vec<EventListener>,
}

impl TobogganNavigationElement {
    const BUTTON_CONFIGS: &'static [(&'static str, &'static str, Command)] = &[
        ("Go to first slide", "home-btn", Command::First),
        ("Previous slide", "prev-btn", Command::Previous),
        ("Pause presentation", "pause-btn", Command::Pause),
        ("Resume presentation", "resume-btn", Command::Resume),
        ("Next slide", "next-btn", Command::Next),
        ("Go to last slide", "last-btn", Command::Last),
    ];

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

    pub fn set_talk(&mut self, talk: Option<TalkResponse>) {
        self.talk = talk;
        if let (Some(title_el), Some(talk)) = (&self.title_el, &self.talk) {
            title_el.set_text_content(Some(&talk.title));
        }
    }

    pub fn set_connection_status(&mut self, status: Option<ConnectionStatus>) {
        self.connection_status = status;
        self.render_connection_status();
    }

    pub fn set_slide_current(&mut self, current: Option<usize>) {
        self.slide_current = current;
        self.render_slide_info();
        self.update_progress_bar();
        self.render_state();
    }

    pub fn set_slide_count(&mut self, count: Option<usize>) {
        self.slide_count = count;
        self.render_slide_info();
        self.update_progress_bar();
        self.render_state();
    }

    pub fn slide_count(&self) -> Option<usize> {
        self.slide_count
    }

    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.duration = duration;
        self.render_duration();

        if matches!(self.state, Some(State::Running { .. })) {
            self.start_duration_timer();
        }
    }

    fn render_state(&mut self) {
        let Some(container) = &self.container else {
            return;
        };

        let mut classes = vec![
            self.state
                .as_ref()
                .map_or("none", |state| state.to_css_class()),
        ];

        let current = self.slide_current.unwrap_or(0);
        let total = self.slide_count.unwrap_or(1);

        if current == 0 {
            classes.push("at-first");
        }
        if current >= total.saturating_sub(1) {
            classes.push("at-last");
        }

        container.set_class_name(&classes.join(" "));
        self.manage_duration_timer();
    }

    fn render_connection_status(&mut self) {
        let Some(el) = &self.connection_status_el else {
            return;
        };

        if let Some(status) = &self.connection_status {
            let class = format!("connection {}", status.to_css_class());
            el.set_class_name(&class);

            match status {
                ConnectionStatus::Reconnecting { .. } | ConnectionStatus::Error { .. } => {
                    el.set_text_content(Some(&status.to_string()));
                }
                _ => el.set_text_content(None),
            }
        } else {
            el.set_class_name("connection none");
            el.set_text_content(None);
        }
    }

    fn render_slide_info(&mut self) {
        let Some(el) = &self.slide_info_el else {
            return;
        };

        if let Some(current) = self.slide_current {
            dom_try!(
                el.set_attribute("data-current", &(current + 1).to_string()),
                "set data-current"
            );
        } else {
            dom_try!(el.remove_attribute("data-current"), "remove data-current");
        }

        if let Some(total) = self.slide_count {
            dom_try!(
                el.set_attribute("data-total", &total.to_string()),
                "set data-total"
            );
        } else {
            dom_try!(el.remove_attribute("data-total"), "remove data-total");
        }

        el.set_text_content(Some(""));
    }

    fn update_progress_bar(&mut self) {
        let Some(progress) = &self.progress_el else {
            return;
        };

        #[allow(clippy::cast_precision_loss)]
        {
            let max = self
                .slide_count
                .map_or(1.0, |count| (count.saturating_sub(1) as f64).max(1.0));
            let value = self.slide_current.map_or(0.0, |i| i as f64);

            progress.set_max(max);
            progress.set_value(value);
        }
    }

    fn render_duration(&mut self) {
        let Some(el) = &self.duration_el else { return };

        let text = self
            .duration
            .map_or_else(String::new, |duration| duration.to_string());
        el.set_text_content(Some(&text));
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

        if let Some(el) = &self.duration_el {
            let el_clone = el.clone();
            let mut seconds = self
                .duration
                .map_or(0, |duration| core::time::Duration::from(duration).as_secs());

            let timer = Interval::new(1000, move || {
                seconds += 1;
                el_clone.set_text_content(Some(&format_duration(seconds)));
            });

            self.duration_timer = Some(timer);
        }
    }

    fn stop_duration_timer(&mut self) {
        if let Some(timer) = self.duration_timer.take() {
            timer.cancel();
        }
    }

    fn setup_button_handlers(&mut self) {
        let Some(tx_cmd) = &self.tx_cmd else { return };
        self.listeners.clear();

        for (btn, (_, _, cmd)) in self.buttons.iter().zip(Self::BUTTON_CONFIGS.iter()) {
            let tx = tx_cmd.clone();
            let cmd = cmd.clone();
            let listener = EventListener::new(btn, "click", move |_| {
                let _ = tx.unbounded_send(cmd.clone());
            });
            self.listeners.push(listener);
        }
    }
}

impl WasmElement for TobogganNavigationElement {
    fn render(&mut self, host: &HtmlElement) {
        let root = dom_try!(
            create_shadow_root_with_style(host, CSS),
            "create shadow root"
        );

        let nav = dom_try!(
            create_and_append_element::<Element>(&root, "nav"),
            "create nav"
        );

        let progress = dom_try!(
            create_and_append_element::<HtmlProgressElement>(&nav, "progress"),
            "create progress"
        );
        progress.set_max(1.0);
        progress.set_value(0.0);

        let title = dom_try!(
            create_and_append_element::<Element>(&nav, "h1"),
            "create title"
        );

        let buttons_container = dom_try!(
            create_and_append_element::<Element>(&nav, "div"),
            "create buttons"
        );
        buttons_container.set_class_name("buttons");

        for (title, css_class, _) in Self::BUTTON_CONFIGS {
            let btn = create_button("", title).unwrap_or_else(|_| {
                create_and_append_element::<HtmlButtonElement>(&buttons_container, "button")
                    .unwrap_throw()
            });
            btn.set_class_name(css_class);
            dom_try!(buttons_container.append_child(&btn), "append button");
            self.buttons.push(btn);
        }

        let status = dom_try!(
            create_and_append_element::<Element>(&nav, "div"),
            "create status"
        );
        status.set_class_name("status");

        let connection = dom_try!(
            create_and_append_element::<Element>(&status, "span"),
            "create connection"
        );
        connection.set_class_name("connection");

        let slide_info = dom_try!(
            create_and_append_element::<Element>(&status, "span"),
            "create slide info"
        );
        slide_info.set_class_name("slide-info");

        let duration = dom_try!(
            create_and_append_element::<Element>(&status, "span"),
            "create duration"
        );
        duration.set_class_name("duration");

        self.container = Some(nav);
        self.progress_el = Some(progress);
        self.title_el = Some(title);
        self.connection_status_el = Some(connection);
        self.slide_info_el = Some(slide_info);
        self.duration_el = Some(duration);

        self.render_state();
        self.render_connection_status();
        self.render_slide_info();
        self.render_duration();

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
