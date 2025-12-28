use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;
use toboggan_client::{TobogganApi, TobogganConfig};
use toboggan_core::{Notification, Slide, TalkResponse};
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::connection_handler::ConnectionHandler;
use crate::events::AppEvent;
use crate::state::AppState;
use crate::ui::PresenterComponents;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);
const TICK_DELAY: Duration = Duration::from_millis(250);

pub struct App {
    state: Rc<RefCell<AppState>>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    connection_handler: ConnectionHandler,
    api: TobogganApi,
}

impl App {
    /// Create a new TUI application with pre-fetched data.
    #[must_use]
    pub fn new(
        config: &TobogganConfig,
        api: TobogganApi,
        talk: TalkResponse,
        slides: Vec<Slide>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = AppState::new(talk, slides);
        let state = Rc::new(RefCell::new(state));
        let connection_handler = ConnectionHandler::new(config.clone(), event_tx.clone());

        debug!("Config: {config:#?}");

        Self {
            state,
            event_rx,
            event_tx,
            connection_handler,
            api,
        }
    }

    /// Run the TUI application.
    ///
    /// # Errors
    ///
    /// Returns an error if the application fails to run.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        info!("Starting Toboggan TUI Presenter");
        self.connection_handler.start();
        self.start_keyboard_handler();

        let mut last_tick = Instant::now();
        'main_loop: loop {
            self.render_app(terminal).context("render")?;

            // Handle crossterm events (resize, etc.)
            if crossterm::event::poll(EVENT_POLL_TIMEOUT).context("poll event")?
                && let Ok(Event::Resize(cols, rows)) = event::read()
            {
                let mut state = self.state.borrow_mut();
                state.terminal_size = (cols, rows);
            }

            // Handle app events
            while let Ok(app_event) = self.event_rx.try_recv() {
                // Intercept TalkChange to trigger refetch
                if let AppEvent::NotificationReceived(ref notification) = app_event
                    && matches!(notification, Notification::TalkChange { .. })
                {
                    info!("📝 TalkChange received - spawning refetch task");
                    let api = self.api.clone();
                    let tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        match tokio::try_join!(api.talk(), api.slides()) {
                            Ok((talk, slides)) => {
                                info!("✅ Talk and slides refetched");
                                let _ = tx.send(AppEvent::TalkAndSlidesRefetched(
                                    Box::new(talk),
                                    slides.slides,
                                ));
                            }
                            Err(err) => {
                                let _ =
                                    tx.send(AppEvent::Error(format!("Failed to refetch: {err}")));
                            }
                        }
                    });
                }

                let mut state = self.state.borrow_mut();
                if state
                    .handle_event(app_event, &self.connection_handler)
                    .is_break()
                {
                    break 'main_loop; // Quit requested
                }
            }

            // Send tick event periodically
            if last_tick.elapsed() >= TICK_DELAY {
                let _ = self.event_tx.send(AppEvent::Tick);
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    fn render_app(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let shared_state = Rc::clone(&self.state);
        terminal
            .draw(move |frame| {
                let components = PresenterComponents::default();
                let mut state = shared_state.borrow_mut();
                frame.render_stateful_widget(&components, frame.area(), &mut state);
            })
            .context("drawing")?;

        Ok(())
    }

    fn start_keyboard_handler(&self) {
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(event) = event::read()
                    && let Event::Key(key) = event
                    && tx.send(AppEvent::Key(key)).is_err()
                {
                    break;
                }
            }
        });
    }
}
