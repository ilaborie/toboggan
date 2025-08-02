use std::collections::HashMap;

use iced::widget::{button, column, container, row, text};
use iced::{Application, Element, Length, Theme, keyboard, Command};
use toboggan_core::{ClientId, Command as TobogganCommand, Slide, SlideId, State, Talk};
use tracing::{error, info};

use crate::config::Config;
use crate::messages::Message;
use crate::services::slide_client::SlideClient;
use crate::services::websocket::WebSocketClient;
use crate::ui::slide_view::render_slide;

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

pub struct TobogganApp {
    config: Config,
    connection_status: ConnectionStatus,
    current_slide: Option<SlideId>,
    slides: HashMap<SlideId, Slide>,
    presentation_state: Option<State>,
    talk: Option<Talk>,
    error_message: Option<String>,
    client_id: ClientId,
    websocket_client: Option<WebSocketClient>,
    slide_client: SlideClient,
}

impl Application for TobogganApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = Config;

    fn new(config: Config) -> (Self, Command<Message>) {
        let client_id = ClientId::new();
        let slide_client = SlideClient::new(&config.websocket_url);

        let app = Self {
            config,
            connection_status: ConnectionStatus::Disconnected,
            current_slide: None,
            slides: HashMap::new(),
            presentation_state: None,
            talk: None,
            error_message: None,
            client_id,
            websocket_client: None,
            slide_client,
        };

        (
            app,
            Command::perform(async {}, |()| Message::ConnectToServer),
        )
    }

    fn title(&self) -> String {
        if let Some(talk) = &self.talk {
            format!("Toboggan - {}", talk.title)
        } else {
            "Toboggan Desktop Client".to_string()
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WebSocketConnected => {
                self.connection_status = ConnectionStatus::Connected;
                self.error_message = None;
                info!("Connected to WebSocket");
                // Fetch slides after connection
                let slide_client = self.slide_client.clone();
                return Command::perform(
                    async move {
                        match slide_client.fetch_slides().await {
                            Ok(slides) => Message::SlidesLoaded(slides),
                            Err(fetch_error) => Message::WebSocketError(format!(
                                "Failed to fetch slides: {fetch_error}"
                            )),
                        }
                    },
                    |msg| msg,
                );
            }
            Message::WebSocketDisconnected => {
                self.connection_status = ConnectionStatus::Disconnected;
                info!("Disconnected from WebSocket");
            }
            Message::WebSocketError(error) => {
                self.connection_status = ConnectionStatus::Error(error.clone());
                self.error_message = Some(error);
                error!("WebSocket error");
            }
            Message::NotificationReceived(notification) => {
                self.handle_notification(notification);
            }
            Message::FirstSlide => {
                return self.send_command(&TobogganCommand::First);
            }
            Message::PreviousSlide => {
                return self.send_command(&TobogganCommand::Previous);
            }
            Message::NextSlide => {
                return self.send_command(&TobogganCommand::Next);
            }
            Message::LastSlide => {
                return self.send_command(&TobogganCommand::Last);
            }
            Message::PlayPresentation => {
                return self.send_command(&TobogganCommand::Resume);
            }
            Message::PausePresentation => {
                return self.send_command(&TobogganCommand::Pause);
            }
            Message::GoToSlide(slide_id) => {
                return self.send_command(&TobogganCommand::GoTo(slide_id));
            }
            Message::ConnectToServer => {
                self.connection_status = ConnectionStatus::Connecting;
                let url = self.config.websocket_url.clone();
                let client_id = self.client_id;

                return Command::perform(
                    async move {
                        let mut client = WebSocketClient::new(client_id);
                        match client.connect(&url).await {
                            Ok(_receiver) => Ok(()),
                            Err(connect_error) => Err(connect_error.to_string()),
                        }
                    },
                    |result| match result {
                        Ok(()) => Message::WebSocketConnected,
                        Err(error_msg) => Message::WebSocketError(error_msg),
                    },
                );
            }
            Message::Tick | Message::WindowResized => {
                // Handle periodic updates and window resize
            }
            Message::SlidesLoaded(slides) => {
                self.slides = slides;
                info!("Loaded {} slides", self.slides.len());
                // Also fetch the talk metadata
                let slide_client = self.slide_client.clone();
                return Command::perform(
                    async move {
                        match slide_client.fetch_talk().await {
                            Ok(talk) => Message::TalkLoaded(talk),
                            Err(talk_error) => Message::WebSocketError(format!(
                                "Failed to fetch talk: {talk_error}"
                            )),
                        }
                    },
                    |msg| msg,
                );
            }
            Message::TalkLoaded(talk) => {
                self.talk = Some(talk);
                info!("Talk loaded");
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let status_text = match &self.connection_status {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Error(err) => &format!("Error: {err}"),
        };

        let status_bar = row![
            text(status_text),
            text(format!("URL: {}", self.config.websocket_url)),
        ]
        .spacing(20);

        let controls = row![
            button("First").on_press(Message::FirstSlide),
            button("Previous").on_press(Message::PreviousSlide),
            button("Next").on_press(Message::NextSlide),
            button("Last").on_press(Message::LastSlide),
            button("Play").on_press(Message::PlayPresentation),
            button("Pause").on_press(Message::PausePresentation),
        ]
        .spacing(10);

        let slide_content: Element<Message> = if let Some(slide_id) = &self.current_slide {
            if let Some(slide) = self.slides.get(slide_id) {
                container(render_slide(slide))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                container(text("Loading slide..."))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .into()
            }
        } else {
            container(text("No slide selected"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
        };

        let content = column![status_bar, slide_content, controls,]
            .spacing(20)
            .padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| match (key.as_ref(), modifiers) {
            (keyboard::Key::Named(keyboard::key::Named::ArrowRight), _) => Some(Message::NextSlide),
            (keyboard::Key::Named(keyboard::key::Named::ArrowLeft), _) => {
                Some(Message::PreviousSlide)
            }
            (keyboard::Key::Named(keyboard::key::Named::Home), _) => Some(Message::FirstSlide),
            (keyboard::Key::Named(keyboard::key::Named::End), _) => Some(Message::LastSlide),
            (keyboard::Key::Named(keyboard::key::Named::Space), _) => {
                Some(Message::PlayPresentation)
            }
            (keyboard::Key::Named(keyboard::key::Named::Escape), _) => {
                Some(Message::PausePresentation)
            }
            _ => None,
        })
    }
}

impl TobogganApp {
    fn handle_notification(&mut self, notification: toboggan_core::Notification) {
        use toboggan_core::Notification;

        match notification {
            Notification::State { state, .. } => {
                self.presentation_state = Some(state.clone());
                self.current_slide = state.current();
                info!("Received state update: {:?}", state);
            }
            Notification::Pong { .. } => {
                // Heartbeat response
            }
            Notification::Error { message, .. } => {
                self.error_message = Some(message);
            }
        }
    }

    fn send_command(&self, command: &TobogganCommand) -> Command<Message> {
        if let Some(client) = &self.websocket_client {
            if let Err(send_error) = client.send_command(command.clone()) {
                error!("Failed to send command: {}", send_error);
                let error_message = format!("Failed to send command: {send_error}");
                return Command::perform(async {}, move |()| {
                    Message::WebSocketError(error_message)
                });
            }
        }
        info!("Sending command: {:?}", command);
        Command::none()
    }
}
