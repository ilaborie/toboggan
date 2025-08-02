use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use toboggan_core::{Command, Notification, State};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::config::Config;
use crate::events::AppEvent;
use crate::state::{AppState, ConnectionStatus};
use crate::terminal::{TerminalType, restore_terminal, setup_terminal};
use crate::ui::render_ui;
use crate::websocket::WebSocketClient;

pub struct App {
    state: AppState,
    terminal: TerminalType,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    /// Create a new TUI application.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal setup fails.
    pub fn new(config: Config) -> Result<Self> {
        let terminal = setup_terminal()?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = AppState::new(config);

        Ok(Self {
            state,
            terminal,
            event_rx,
            event_tx,
        })
    }

    /// Run the TUI application.
    ///
    /// # Errors
    ///
    /// Returns an error if the application fails to run.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Toboggan TUI");

        // Start WebSocket client
        self.start_websocket_client();

        // Start keyboard event handler
        self.start_keyboard_handler();

        // Start tick handler
        self.start_tick_handler();

        // Main event loop
        let mut last_tick = Instant::now();
        'main_loop: loop {
            // Render UI
            self.terminal.draw(|frame| render_ui(frame, &self.state))?;

            // Handle events with timeout
            let timeout = Duration::from_millis(50);
            if crossterm::event::poll(timeout)? {
                // Handle crossterm events (resize, etc.)
                if let Ok(Event::Resize(cols, rows)) = event::read() {
                    self.state.terminal_size = (cols, rows);
                }
            }

            // Handle app events
            while let Ok(app_event) = self.event_rx.try_recv() {
                if self.handle_event(app_event).await? {
                    break 'main_loop; // Quit requested
                }
            }

            // Send tick event periodically
            if last_tick.elapsed() >= Duration::from_millis(250) {
                let _ = self.event_tx.send(AppEvent::Tick);
                last_tick = Instant::now();
            }
        }

        self.cleanup()
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<bool> {
        debug!("Handling event: {:?}", event);

        match event {
            AppEvent::Key(key) => {
                match key.code {
                    KeyCode::Char('q' | 'Q') => return Ok(true),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(true);
                    }
                    KeyCode::Char('h' | 'H' | '?') => {
                        self.state.show_help = !self.state.show_help;
                    }
                    KeyCode::Char('c' | 'C') => {
                        self.state.error_message = None;
                    }
                    KeyCode::Char('r' | 'R') => {
                        self.start_websocket_client();
                    }
                    KeyCode::Char('f' | 'F') | KeyCode::Home => {
                        self.send_command(Command::First);
                    }
                    KeyCode::Char('p' | 'P') | KeyCode::Left => {
                        self.send_command(Command::Previous);
                    }
                    KeyCode::Char('n' | 'N') | KeyCode::Right => {
                        self.send_command(Command::Next);
                    }
                    KeyCode::Char('l' | 'L') | KeyCode::End => {
                        self.send_command(Command::Last);
                    }
                    KeyCode::Char(' ') => {
                        // Toggle play/pause based on current state
                        if let Some(state) = &self.state.presentation_state {
                            match state {
                                State::Running { .. } => {
                                    self.send_command(Command::Pause);
                                }
                                State::Paused { .. } => {
                                    self.send_command(Command::Resume);
                                }
                                State::Init | State::Done { .. } => {
                                    self.send_command(Command::Next);
                                }
                            }
                        } else {
                            self.send_command(Command::Next);
                        }
                    }
                    _ => {}
                }
            }
            AppEvent::Connected => {
                self.state.connection_status = ConnectionStatus::Connected;
                info!("WebSocket connected");
            }
            AppEvent::Disconnected => {
                self.state.connection_status = ConnectionStatus::Disconnected;
                info!("WebSocket disconnected");
            }
            AppEvent::ConnectionError(error) => {
                self.state.connection_status = ConnectionStatus::Error(error.clone());
                self.state.error_message = Some(error);
            }
            AppEvent::NotificationReceived(notification) => {
                self.handle_notification(notification)?;
            }
            AppEvent::SlideLoaded(slide_id, slide) => {
                self.state.slides.insert(slide_id, slide);
            }
            AppEvent::SlideLoadError(_slide_id, error) => {
                self.state.error_message = Some(format!("Failed to load slide: {error}"));
            }
            AppEvent::Quit => return Ok(true),
            _ => {}
        }

        Ok(false)
    }

    fn handle_notification(&mut self, notification: Notification) -> Result<()> {
        match notification {
            Notification::State { state, .. } => {
                self.state.update_presentation_state(state);

                // Load slide if we don't have it
                if let Some(slide_id) = &self.state.current_slide {
                    if !self.state.slides.contains_key(slide_id) {
                        self.load_slide(*slide_id);
                    }
                }
            }
            Notification::Pong { .. } => {
                debug!("Received pong");
            }
            Notification::Error { message, .. } => {
                self.state.error_message = Some(message);
            }
            Notification::Blink => todo!(),
        }
        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn send_command(&self, _command: Command) {
        debug!("Sending command: {:?}", _command);
        // In a real implementation, we'd send this through the WebSocket
        // For now, we'll just log it
    }

    fn load_slide(&self, slide_id: toboggan_core::SlideId) {
        debug!("Loading slide: {:?}", slide_id);

        let api_url = format!("{}/api/slides/{:?}", self.state.config.api_url, slide_id);
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            match reqwest::get(&api_url).await {
                Ok(response) => match response.json::<toboggan_core::Slide>().await {
                    Ok(slide) => {
                        let _ = event_tx.send(AppEvent::SlideLoaded(slide_id, slide));
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::SlideLoadError(slide_id, e.to_string()));
                    }
                },
                Err(e) => {
                    let _ = event_tx.send(AppEvent::SlideLoadError(slide_id, e.to_string()));
                }
            }
        });
    }

    fn start_websocket_client(&mut self) {
        self.state.connection_status = ConnectionStatus::Connecting;

        let client = WebSocketClient::new(
            self.event_tx.clone(),
            self.state.config.websocket_url.clone(),
            self.state.config.max_retries,
            self.state.config.retry_delay_ms,
        );

        tokio::spawn(async move {
            client.run().await;
        });
    }

    fn start_keyboard_handler(&self) {
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(event) = event::read() {
                    if let Event::Key(key) = event {
                        if tx.send(AppEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }

    fn start_tick_handler(&self) {
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            loop {
                interval.tick().await;
                if tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });
    }

    fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up Toboggan TUI");
        restore_terminal(&mut self.terminal)?;
        Ok(())
    }
}
