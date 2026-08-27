use std::sync::OnceLock;

use iced::{Element, Subscription, Task, Theme, keyboard, window};
use toboggan_client::{
    CommunicationMessage, ConnectionStatus, TobogganApi, TobogganApiError, TobogganConfig,
    WebSocketClient, refetch_talk_and_slides,
};
use toboggan_core::{
    ClientConfig, Command as TobogganCommand, SlidesResponse, Talk, TalkResponse, accumulate_goto,
    goto_command,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

use crate::actions::AppAction;
use crate::message::Message;
use crate::state::{AppState, parse_slides_markdown};
use crate::{slide_list, views};

// Global channel for forwarding WebSocket messages to Iced
static MESSAGE_CHANNEL: OnceLock<broadcast::Sender<CommunicationMessage>> = OnceLock::new();

pub struct App {
    config: TobogganConfig,
    state: AppState,
    /// The channel into the live socket, and so also the answer to "is one
    /// live". There is no handle to the socket itself: `WebSocketClient` is
    /// moved into the task that drives it. A field held one and was never
    /// assigned, which made the leak `handle_connect` now guards against look
    /// as though something were keeping track.
    cmd_sender: Option<mpsc::UnboundedSender<TobogganCommand>>,
    api: TobogganApi,
}

impl App {
    /// Creates a new app instance.
    ///
    /// # Panics
    /// Panics if the message channel has already been initialized.
    pub fn new(config: TobogganConfig) -> (Self, Task<Message>) {
        let api_client = TobogganApi::new(config.api_url());

        // Initialize the global message channel for WebSocket message forwarding
        let (tx, _) = broadcast::channel(1000);
        assert!(
            MESSAGE_CHANNEL.set(tx).is_ok(),
            "Failed to initialize message channel - already initialized"
        );

        let app = Self {
            config,
            state: AppState::default(),
            cmd_sender: None,
            api: api_client.clone(),
        };

        // Load talk and slides immediately, then connect
        let api_for_loading = api_client;
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
                // The first reading. `theme_changes` only reports a *change*,
                // so without this the app follows the desktop from the first
                // time it flips and shows the wrong one until then.
                iced::system::theme().map(Message::SystemThemeChanged),
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

            Message::ToggleFullscreen => self.toggle_fullscreen(),

            Message::KeyPressed(key, modifiers) => self.handle_keyboard(&key, modifiers),

            Message::LinkClicked(url) => {
                // `open::that` shells out to the platform launcher and blocks
                // until it returns, which on macOS is long enough to drop
                // frames — so it goes to a blocking thread, the way the server
                // runs the very same call behind `--open`
                // (`toboggan-server/src/bootstrap.rs:120`).
                info!(?url, "Opening a link in the default browser");
                Task::future(async move {
                    match tokio::task::spawn_blocking(move || open::that(&*url)).await {
                        Ok(Ok(())) => (),
                        Ok(Err(err)) => error!("Could not open the link: {err}"),
                        Err(err) => error!("The task that opens links panicked: {err}"),
                    }
                })
                .discard()
            }

            Message::ToggleTimer => {
                self.state.elapsed.toggle(self.state.now());
                Task::none()
            }

            Message::ResetTimer => {
                self.state.elapsed.restart(self.state.now());
                Task::none()
            }

            Message::ThemeChosen(choice) => {
                self.state.theme_choice = choice;
                Task::none()
            }

            Message::SystemThemeChanged(mode) => {
                self.state.system_mode = mode;
                Task::none()
            }

            // Nothing to update: the tick exists to bring `view` round again so
            // the clock and the elapsed timer redraw. It is the one thing here
            // that moves without the deck moving.
            Message::WindowResized(_, _) | Message::Tick => Task::none(),
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        views::main_view(&self.state)
    }

    #[must_use]
    pub fn theme(&self) -> Theme {
        self.state.theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // `keyboard::listen`, not `event::listen_with`. The latter is handed the
        // capture status and this closure threw it away, so every keystroke
        // reached the keymap even when a widget had already consumed it —
        // typing a space into a text field advanced the deck. `listen` yields
        // only `event::Status::Ignored`, so a focused input swallows its own
        // keys and nothing here has to arbitrate.
        let keyboard_subscription =
            keyboard::listen()
                .with(())
                .filter_map(|((), event)| match event {
                    keyboard::Event::KeyPressed { key, modifiers, .. } => {
                        Some(Message::KeyPressed(key, modifiers))
                    }
                    keyboard::Event::KeyReleased { .. } | keyboard::Event::ModifiersChanged(_) => {
                        None
                    }
                });

        let tick_subscription =
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);

        let websocket_subscription = websocket_message_subscription();

        // The desktop can change its mind about light and dark while the app is
        // open — at sunset, on most machines — and a presenter view that keeps
        // the old one until it is restarted is a presenter view that is wrong
        // exactly when the room's lights change.
        let system_theme_subscription =
            iced::system::theme_changes().map(Message::SystemThemeChanged);

        Subscription::batch(vec![
            keyboard_subscription,
            tick_subscription,
            websocket_subscription,
            system_theme_subscription,
        ])
    }
}

impl App {
    fn handle_connect(&mut self) -> Task<Message> {
        // Every call spawns a socket and a forwarding task and never stops the
        // one before it, and both would pump into the single global broadcast —
        // so a second `Connect` while one is live used to double every
        // notification. `cmd_sender` is `Some` exactly while a socket is up,
        // which makes it the guard.
        if self.cmd_sender.is_some() {
            info!("Already connected; ignoring the request to connect again");
            return Task::none();
        }

        info!("Connecting to server...");
        let (tx_cmd, rx_cmd) = mpsc::unbounded_channel();
        let (mut ws_client, mut rx_msg) =
            WebSocketClient::new(tx_cmd.clone(), rx_cmd, "Desktop", self.config.websocket());

        self.cmd_sender = Some(tx_cmd);

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

    /// Drops the connection and stays dropped.
    ///
    /// This used to queue a `Connect` 100 ms later, which made the only control
    /// for it a button that could not do what its label said: "Reconnect" sent
    /// `Disconnect` and was reconnected *for* it, so there was no way to stand
    /// down a client that should not be driving the deck. The footer already
    /// offers a `Connect` button whenever the status is `Closed`, so the way
    /// back is right there.
    fn handle_disconnect(&mut self) -> Task<Message> {
        info!("Disconnecting from server...");
        self.cmd_sender = None;
        self.state.connection_status = ConnectionStatus::Closed;
        Task::none()
    }

    fn handle_talk_loaded(&mut self, talk_response: &TalkResponse) -> Task<Message> {
        info!("Talk loaded: {}", talk_response.title);
        // For now, create a simplified talk from the response
        let talk = Talk {
            title: talk_response.title.clone(),
            date: talk_response.date,
            footer: talk_response.footer.clone(),
            head: talk_response.head.clone(),
            typst_preamble: None,
            lang: talk_response.lang.clone(),
            default_terminal_cwd: None,
            source_dir: None,
            slides: vec![], // We'll load slides separately
        };
        self.state.talk = Some(talk);
        // Store step counts from server
        self.state
            .step_counts
            .clone_from(&talk_response.step_counts);
        self.state.durations.clone_from(&talk_response.durations);
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
            typst_preamble: None,
            lang: talk_response.lang.clone(),
            default_terminal_cwd: None,
            source_dir: None,
            slides: slides_response.slides.clone(),
        };
        self.state.talk = Some(talk);

        // Store all slides in the Vec
        self.state.slides.clone_from(&slides_response.slides);

        // Store step counts from server
        self.state
            .step_counts
            .clone_from(&talk_response.step_counts);
        self.state.durations.clone_from(&talk_response.durations);

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
            typst_preamble: None,
            lang: talk_response.lang.clone(),
            default_terminal_cwd: None,
            source_dir: None,
            slides: slides_response.slides.clone(),
        };
        self.state.talk = Some(talk);
        self.state.slides.clone_from(&slides_response.slides);

        // Store step counts from server
        self.state
            .step_counts
            .clone_from(&talk_response.step_counts);
        self.state.durations.clone_from(&talk_response.durations);

        // Parse and cache markdown for all slides
        self.state.cached_markdown = parse_slides_markdown(&slides_response.slides);

        // Now update state atomically with the fresh data
        self.state.presentation_state = Some(state.clone());
        if let Some(slide_id) = state.current() {
            self.state.current_slide = Some(slide_id);
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
            CommunicationMessage::StateChange { state, .. } => {
                debug!("State change received: {:?}", state);
                self.state.presentation_state = Some(state.clone());
                if let Some(slide_id) = state.current() {
                    // The deck is running, so the talk has begun. Idempotent —
                    // this arrives again on every reveal, and the clock must not
                    // restart on every space bar.
                    self.state.elapsed.start_if_idle(self.state.now());
                    let moved = self.state.current_slide != Some(slide_id);
                    self.state.current_slide = Some(slide_id);

                    // Ensure slides are loaded from talk data
                    if let Some(talk) = &self.state.talk
                        && self.state.slides.is_empty()
                        && !talk.slides.is_empty()
                    {
                        self.state.slides = talk.slides.clone();
                    }

                    // Only when the deck actually changed slide: a reveal keeps
                    // the highlight where it is, and snapping on every space bar
                    // would fight a presenter who had scrolled the list to look
                    // ahead.
                    if moved {
                        return self.snap_to_current_slide();
                    }
                }
                Task::none()
            }
            CommunicationMessage::TalkChange { state, .. } => {
                info!("Presentation updated, reloading talk and slides");

                // DON'T update state immediately - wait for data to be fetched
                // Use shared refetch_talk_and_slides utility
                let api = self.api.clone();
                let state_for_update = state;
                Task::perform(
                    async move {
                        let result = refetch_talk_and_slides(&api).await;
                        (result, state_for_update)
                    },
                    |(result, state)| match result {
                        Ok((talk, slides)) => {
                            // Wrap slides in SlidesResponse for compatibility
                            let slides_response = SlidesResponse { slides };
                            Message::TalkChangeComplete(talk, slides_response, state)
                        }
                        Err(err) => Message::LoadError(err.to_string()),
                    },
                )
            }
            CommunicationMessage::Error { error } => {
                error!("WebSocket error: {}", error);
                self.state.error_message = Some(error);
                Task::none()
            }
            // The grant decides whether the keys do anything, so it reaches the
            // UI rather than stopping here.
            CommunicationMessage::Registered { role, .. } => {
                self.state.role = Some(role);
                Task::none()
            }
            CommunicationMessage::ClientConnected { .. }
            | CommunicationMessage::ClientDisconnected { .. } => Task::none(),
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

    /// Puts the window in or out of fullscreen.
    ///
    /// `state.fullscreen` used to be flipped here and read nowhere, so `F11`
    /// did nothing — while the help panel this crate's `AppAction` refactor
    /// generates advertised it, which is the exact class of drift that refactor
    /// exists to prevent.
    fn toggle_fullscreen(&mut self) -> Task<Message> {
        self.state.fullscreen = !self.state.fullscreen;
        let mode = if self.state.fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };
        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    /// Brings the slide list back to the row the deck is on.
    ///
    /// The list is 42 entries on an ordinary deck and the viewport holds about
    /// twenty, so without this the highlight simply left the screen as soon as
    /// the talk got past the first section — and the sidebar became a thing you
    /// scrolled to find out where you were.
    fn snap_to_current_slide(&self) -> Task<Message> {
        let rows = slide_list::rows(&self.state.slides, self.state.current_slide);
        let Some(y) = slide_list::current_row_fraction(&rows) else {
            return Task::none();
        };
        iced::widget::operation::snap_to(
            views::sidebar::SLIDE_LIST_ID,
            // Only the vertical axis: the list does not scroll sideways, and
            // saying so leaves any horizontal offset alone.
            iced::widget::scrollable::RelativeOffset {
                x: None,
                y: Some(y),
            },
        )
    }

    fn handle_keyboard(
        &mut self,
        key: &keyboard::Key,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        let Some(action) = AppAction::from_key(key, modifiers) else {
            return Task::none();
        };
        if self.state.show_help && !action.ignores_help() {
            return Task::none();
        }

        // Digits accumulate into a slide number and `Enter` jumps to it; the
        // arithmetic is `toboggan_core::goto`'s, shared with the TUI and both
        // web clients rather than copied a fourth time. Anything else abandons
        // a half-typed number rather than leaving it to land on some later
        // `Enter` — the same rule the TUI applies.
        match action {
            AppAction::Digit(digit) => {
                self.state.goto_target = accumulate_goto(self.state.goto_target, digit);
                return Task::none();
            }
            AppAction::GotoTyped => {
                let Some(number) = self.state.goto_target.take() else {
                    return Task::none();
                };
                return self.send_command(goto_command(number));
            }
            _ => self.state.goto_target = None,
        }

        if let Some(command) = action.command() {
            return self.send_command(command);
        }

        match action {
            AppAction::ToggleTimer => self.state.elapsed.toggle(self.state.now()),
            AppAction::ResetTimer => self.state.elapsed.restart(self.state.now()),
            AppAction::ToggleHelp => {
                self.state.show_help = !self.state.show_help;
            }
            AppAction::ToggleSidebar => {
                self.state.show_sidebar = !self.state.show_sidebar;
            }
            AppAction::ToggleFullscreen => return self.toggle_fullscreen(),
            // One key for both overlays, closing whichever is up.
            AppAction::CloseOverlay => {
                if self.state.show_help {
                    self.state.show_help = false;
                } else {
                    self.state.error_message = None;
                }
            }
            AppAction::Quit => return iced::exit(),
            // Both handled above: the goto pair before `command()`, the rest by
            // it.
            AppAction::Digit(_)
            | AppAction::GotoTyped
            | AppAction::First
            | AppAction::Previous
            | AppAction::Next
            | AppAction::Last
            | AppAction::PreviousStep
            | AppAction::NextStep
            | AppAction::Blink => {}
        }
        Task::none()
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
