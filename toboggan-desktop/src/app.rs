use std::sync::OnceLock;

use iced::{Element, Subscription, Task, Theme, event, keyboard};
use toboggan_client::{
    CommunicationMessage, ConnectionStatus, TobogganApi, TobogganApiError, TobogganConfig,
    WebSocketClient,
};
use toboggan_core::{
    ClientConfig, ClientId, Command as TobogganCommand, SlidesResponse, Talk, TalkResponse,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

use crate::message::Message;
use crate::state::{AppState, parse_slides_markdown};
use crate::views;

// Global channel for forwarding WebSocket messages to Iced
static MESSAGE_CHANNEL: OnceLock<broadcast::Sender<CommunicationMessage>> = OnceLock::new();

pub struct App {
    config: TobogganConfig,
    state: AppState,
    websocket_client: Option<WebSocketClient>,
    cmd_sender: Option<mpsc::UnboundedSender<TobogganCommand>>,
    api: TobogganApi,
    client_id: ClientId,
}

impl App {
    /// Creates a new app instance.
    ///
    /// # Panics
    /// Panics if the message channel has already been initialized.
    pub fn new(config: TobogganConfig) -> (Self, Task<Message>) {
        let api_client = TobogganApi::new(config.api_url());
        let client_id = config.client_id();

        // Initialize the global message channel for WebSocket message forwarding
        let (tx, _) = broadcast::channel(1000);
        assert!(
            MESSAGE_CHANNEL.set(tx).is_ok(),
            "Failed to initialize message channel - already initialized"
        );

        let app = Self {
            config,
            state: AppState::default(),
            websocket_client: None,
            cmd_sender: None,
            api: api_client.clone(),
            client_id,
        };

        // Load talk and slides immediately, then connect
        let api_for_loading = api_client.clone();
        (
            app,
            Task::batch([
                Task::perform(
                    async move {
                        let talk = api_for_loading.talk().await?;
                        let slides = api_for_loading.slides().await?;
                        Ok::<_, TobogganApiError>((talk, slides))
                    },
                    |result| match result {
                        Ok((talk, slides)) => Message::TalkAndSlidesLoaded(talk, slides),
                        Err(err) => Message::LoadError(err.to_string()),
                    },
                ),
                Task::perform(async {}, |()| Message::Connect),
            ]),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Connect => self.handle_connect(),

            Message::Disconnect => self.handle_disconnect(),

            Message::TalkLoaded(talk_response) => self.handle_talk_loaded(&talk_response),

            Message::TalkAndSlidesLoaded(talk_response, slides_response) => {
                self.handle_talk_and_slides_loaded(&talk_response, &slides_response)
            }

            Message::TalkChangeComplete(talk_response, slides_response, state) => {
                self.handle_talk_change_complete(&talk_response, &slides_response, &state)
            }

            Message::Communication(message) => self.handle_websocket_message(message),

            Message::SlideLoaded(id, slide) => {
                debug!("Slide loaded: {}", id);
                if let Some(existing_slide) = self.state.slides.get_mut(id) {
                    *existing_slide = slide;
                } else {
                    // Extend the Vec if needed
                    self.state.slides.resize(id + 1, slide.clone());
                    if let Some(target_slide) = self.state.slides.get_mut(id) {
                        *target_slide = slide;
                    }
                }
                Task::none()
            }

            Message::LoadError(error) => {
                error!("Load error: {}", error);
                self.state.error_message = Some(error);
                Task::none()
            }

            Message::SendCommand(command) => self.send_command(command),

            Message::ToggleHelp => {
                self.state.show_help = !self.state.show_help;
                Task::none()
            }

            Message::ToggleSidebar => {
                self.state.show_sidebar = !self.state.show_sidebar;
                Task::none()
            }

            Message::ToggleFullscreen => {
                self.state.fullscreen = !self.state.fullscreen;
                Task::none()
            }

            Message::KeyPressed(key, modifiers) => self.handle_keyboard(key, modifiers),

            Message::LinkClicked(url) => {
                info!(?url, "Link clicked");
                // Could open URL in browser if needed
                Task::none()
            }

            Message::WindowResized(_, _) | Message::Tick => Task::none(),
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        views::main_view(&self.state)
    }

    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_subscription = event::listen_with(|event, _status, _window| {
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            {
                Some(Message::KeyPressed(key, modifiers))
            } else {
                None
            }
        });

        let tick_subscription =
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);

        let websocket_subscription = websocket_message_subscription();

        Subscription::batch(vec![
            keyboard_subscription,
            tick_subscription,
            websocket_subscription,
        ])
    }
}

impl App {
    fn handle_connect(&mut self) -> Task<Message> {
        info!("Connecting to server...");
        let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();
        let (mut ws_client, mut rx_msg) = WebSocketClient::new(
            tx_cmd.clone(),
            rx_cmd,
            self.client_id,
            self.config.websocket(),
        );

        self.cmd_sender = Some(tx_cmd.clone());

        // Send register command
        let _ = tx_cmd.send(TobogganCommand::Register {
            client: self.client_id,
        });

        // Start WebSocket connection and message forwarding in background
        tokio::spawn(async move {
            // Start connection
            ws_client.connect().await;

            // Forward all WebSocket messages to Iced via broadcast channel
            while let Some(msg) = rx_msg.recv().await {
                info!("Received WebSocket message: {:?}", msg);

                // Forward the message to the global broadcast channel
                if let Some(sender) = MESSAGE_CHANNEL.get()
                    && let Err(send_error) = sender.send(msg)
                {
                    error!("Failed to forward WebSocket message: {}", send_error);
                }
            }
        });

        Task::none()
    }

    fn handle_disconnect(&mut self) -> Task<Message> {
        info!("Disconnecting from server...");
        self.websocket_client = None;
        self.cmd_sender = None;
        self.state.connection_status = ConnectionStatus::Closed;

        // Auto-reconnect after disconnect
        Task::perform(
            async { tokio::time::sleep(tokio::time::Duration::from_millis(100)).await },
            |()| Message::Connect,
        )
    }

    fn handle_talk_loaded(&mut self, talk_response: &TalkResponse) -> Task<Message> {
        info!("Talk loaded: {}", talk_response.title);
        // For now, create a simplified talk from the response
        let talk = Talk {
            title: talk_response.title.clone(),
            date: talk_response.date,
            footer: talk_response.footer.clone(),
            head: talk_response.head.clone(),
            slides: vec![], // We'll load slides separately
        };
        self.state.talk = Some(talk);
        Task::none()
    }

    fn handle_talk_and_slides_loaded(
        &mut self,
        talk_response: &TalkResponse,
        slides_response: &SlidesResponse,
    ) -> Task<Message> {
        info!(
            "Talk and slides loaded: {} ({} slides)",
            talk_response.title,
            slides_response.slides.len()
        );
        // Create talk with actual slides
        let talk = Talk {
            title: talk_response.title.clone(),
            date: talk_response.date,
            footer: talk_response.footer.clone(),
            head: talk_response.head.clone(),
            slides: slides_response.slides.clone(),
        };
        self.state.talk = Some(talk);

        // Store all slides in the Vec
        self.state.slides.clone_from(&slides_response.slides);

        // Parse and cache markdown for all slides
        self.state.cached_markdown = parse_slides_markdown(&slides_response.slides);

        Task::none()
    }

    fn handle_talk_change_complete(
        &mut self,
        talk_response: &TalkResponse,
        slides_response: &SlidesResponse,
        state: &toboggan_core::State,
    ) -> Task<Message> {
        info!(
            "📝 Talk change complete: {} ({} slides)",
            talk_response.title,
            slides_response.slides.len()
        );

        // Update talk and slides
        let talk = Talk {
            title: talk_response.title.clone(),
            date: talk_response.date,
            footer: talk_response.footer.clone(),
            head: talk_response.head.clone(),
            slides: slides_response.slides.clone(),
        };
        self.state.talk = Some(talk);
        self.state.slides.clone_from(&slides_response.slides);

        // Parse and cache markdown for all slides
        self.state.cached_markdown = parse_slides_markdown(&slides_response.slides);

        // Now update state atomically with the fresh data
        self.state.presentation_state = Some(state.clone());
        if let Some(slide_idx) = state.current() {
            self.state.current_slide_index = Some(slide_idx);
        }

        Task::none()
    }

    fn handle_websocket_message(&mut self, message: CommunicationMessage) -> Task<Message> {
        match message {
            CommunicationMessage::ConnectionStatusChange { status } => {
                self.state.connection_status = status.clone();
                info!("Connection status changed: {:?}", status);

                // Load talk data when connection is established (formerly in handle_connection_status_change)
                if matches!(status, ConnectionStatus::Connected) {
                    let api = self.api.clone();
                    Task::perform(async move { api.talk().await }, |result| match result {
                        Ok(talk) => Message::TalkLoaded(talk),
                        Err(load_error) => Message::LoadError(load_error.to_string()),
                    })
                } else {
                    Task::none()
                }
            }
            CommunicationMessage::StateChange { state } => {
                debug!("State change received: {:?}", state);
                self.state.presentation_state = Some(state.clone());
                if let Some(slide_idx) = state.current() {
                    self.state.current_slide_index = Some(slide_idx);

                    // Ensure slides are loaded from talk data
                    if let Some(talk) = &self.state.talk
                        && self.state.slides.is_empty()
                        && !talk.slides.is_empty()
                    {
                        self.state.slides = talk.slides.clone();
                    }
                }
                Task::none()
            }
            CommunicationMessage::TalkChange { state } => {
                info!("📝 Presentation updated, reloading talk and slides");

                // DON'T update state immediately - wait for data to be fetched
                // Refetch talk and slides from server, then update everything atomically
                let api = self.api.clone();
                let state_for_update = state.clone();
                Task::perform(
                    async move {
                        let talk_result = api.talk().await;
                        let slides_result = api.slides().await;
                        (talk_result, slides_result, state_for_update)
                    },
                    |(talk_result, slides_result, state)| match (talk_result, slides_result) {
                        (Ok(talk), Ok(slides)) => Message::TalkChangeComplete(talk, slides, state),
                        (Err(err), _) | (_, Err(err)) => Message::LoadError(err.to_string()),
                    },
                )
            }
            CommunicationMessage::Error { error } => {
                error!("WebSocket error: {}", error);
                self.state.error_message = Some(error.clone());
                Task::none()
            }
        }
    }

    fn send_command(&mut self, command: TobogganCommand) -> Task<Message> {
        if let Some(sender) = &self.cmd_sender
            && let Err(send_error) = sender.send(command)
        {
            error!("Failed to send command: {}", send_error);
        }
        Task::none()
    }

    fn handle_keyboard(
        &mut self,
        key: keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        match key {
            keyboard::Key::Named(
                keyboard::key::Named::ArrowRight | keyboard::key::Named::Space,
            ) if !self.state.show_help => self.send_command(TobogganCommand::Next),
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft) if !self.state.show_help => {
                self.send_command(TobogganCommand::Previous)
            }
            keyboard::Key::Named(keyboard::key::Named::Home) if !self.state.show_help => {
                self.send_command(TobogganCommand::First)
            }
            keyboard::Key::Named(keyboard::key::Named::End) if !self.state.show_help => {
                self.send_command(TobogganCommand::Last)
            }
            keyboard::Key::Character(character) if character == "h" || character == "?" => {
                self.state.show_help = !self.state.show_help;
                Task::none()
            }
            keyboard::Key::Character(character) if character == "s" && !self.state.show_help => {
                self.state.show_sidebar = !self.state.show_sidebar;
                Task::none()
            }
            keyboard::Key::Character(character)
                if (character == "p" || character == "P") && !self.state.show_help =>
            {
                self.send_command(TobogganCommand::Pause)
            }
            keyboard::Key::Character(character)
                if (character == "r" || character == "R") && !self.state.show_help =>
            {
                self.send_command(TobogganCommand::Resume)
            }
            keyboard::Key::Character(character)
                if (character == "b" || character == "B") && !self.state.show_help =>
            {
                self.send_command(TobogganCommand::Blink)
            }
            keyboard::Key::Named(keyboard::key::Named::F11) => {
                self.state.fullscreen = !self.state.fullscreen;
                Task::none()
            }
            keyboard::Key::Named(keyboard::key::Named::Escape) if self.state.show_help => {
                self.state.show_help = false;
                Task::none()
            }
            keyboard::Key::Named(keyboard::key::Named::Escape)
                if self.state.error_message.is_some() =>
            {
                self.state.error_message = None;
                Task::none()
            }
            keyboard::Key::Character(character) if character == "q" && modifiers.command() => {
                iced::window::close(iced::window::Id::unique())
            }
            _ => Task::none(),
        }
    }
}

// Create a subscription for WebSocket messages
fn websocket_message_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        async_stream::stream! {
            if let Some(channel) = MESSAGE_CHANNEL.get() {
                let mut rx = channel.subscribe();

                loop {
                    if let Ok(message) = rx.recv().await {
                        yield Message::Communication(message);
                    }
                }
            }
        }
    })
}
