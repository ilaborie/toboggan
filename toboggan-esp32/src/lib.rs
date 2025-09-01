use anyhow::Context;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::hal::gpio::{Gpio39, Gpio40, Gpio41, Output, PinDriver};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals};
use log::{error, info};
use mipidsi::TestImage;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Import configuration constants
use crate::config::{display::BUFFER_SIZE, timing::MAIN_LOOP_POLL_INTERVAL};

mod wifi;
pub use self::wifi::*;

mod api;
pub use self::api::*;

mod websocket;
pub use self::websocket::*;

mod display;
pub use self::display::*;

mod state;
pub use self::state::*;

mod led;
pub use self::led::*;

mod display_manager;
pub use self::display_manager::*;

mod config;

// Configuration from environment variables
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const TOBOGGAN_HOST: &str = env!("TOBOGGAN_HOST");

/// Run the main application with synchronous threading model
///
/// # Errors
/// Returns error if application initialization fails, display initialization fails,
/// or main loop encounters unrecoverable errors
///
/// # Panics
/// Panics if thread spawning fails for `WiFi`, API, or WebSocket threads
#[allow(clippy::too_many_lines)]
pub fn run(peripherals: Peripherals, sysloop: EspSystemEventLoop) -> anyhow::Result<()> {
    info!("👋 Hello - Starting synchronous ESP32 application");

    let port = env!("TOBOGGAN_PORT")
        .parse::<u16>()
        .context("Expected a numeric port")?;

    let Peripherals {
        pins, spi2, modem, ..
    } = peripherals;

    // Create state diff channel for efficient updates
    let (diff_sender, diff_receiver) = mpsc::channel::<AppStateDiff>();

    // Create talk data channel
    let (talk_data_sender, talk_data_receiver) = mpsc::channel::<TalkData>();

    // Initialize state manager with diff channel (starts in Booting state by default)
    let mut state_manager = StateManager::new(diff_sender.clone());

    // Initialize display
    let mut buffer = [0_u8; BUFFER_SIZE];
    let display = display(
        spi2,
        pins.gpio7,  // sclk
        pins.gpio6,  // sdo
        pins.gpio5,  // dc
        pins.gpio4,  // cs
        pins.gpio48, // reset
        &mut buffer,
    )
    .context("display init")?;

    // Set up backlight
    let mut backlight = PinDriver::output(pins.gpio47).context("backlight")?;
    backlight.set_high().context("activate backlight")?;

    // Initialize LED pins
    let mut red = PinDriver::output(pins.gpio39).context("initialize red LED pin")?;
    let mut green = PinDriver::output(pins.gpio40).context("initialize green LED pin")?;
    let mut blue = PinDriver::output(pins.gpio41).context("initialize blue LED pin")?;

    // Set initial LED state for Booting
    update_leds_normal(&AppState::Booting, &mut red, &mut green, &mut blue)?;

    // Initialize display with Booting state
    let mut display_manager = DisplayManager::new(display).context("create display manager")?;
    let mut talk_data: Option<TalkData> = None;
    if let Err(error) =
        display_manager.update_display(state_manager.current_state(), talk_data.as_ref())
    {
        log::warn!("Failed to initialize display: {error:?}");
    }

    // Blink timer state for transient LED effects
    let mut blink_end_time: Option<std::time::Instant> = None;

    // Deduplication - track last few diffs to prevent rapid cycling
    let mut last_diff: Option<AppStateDiff> = None;

    // Show test image initially
    let img = TestImage::<Rgb565>::new();
    if let Err(error) = img.draw(&mut display_manager.display) {
        log::error!("Failed to display test image: {error:?}");
    }

    info!("🚀 Starting main application loop");

    // Start WiFi connection immediately since we're in Booting state
    spawn_wifi_thread(diff_sender.clone(), modem, sysloop)?;

    state_manager.transition_to(AppState::Connecting {
        ssid: WIFI_SSID.to_string(),
    });

    let mut api_thread_started = false;
    let mut websocket_thread_started = false;

    // Main loop: handle differential updates and transient effects
    loop {
        // Use shorter timeout when blinking to ensure smooth LED animation
        let timeout = if blink_end_time.is_some() {
            Duration::from_millis(100) // 100ms for smooth blinking
        } else {
            MAIN_LOOP_POLL_INTERVAL
        };
        
        // Check for state diffs with dynamic timeout
        match diff_receiver.recv_timeout(timeout) {
            Ok(diff) => {
                // Skip duplicate diffs to prevent loops (except Blink which is always processed)
                if !matches!(diff, AppStateDiff::Blink) && last_diff.as_ref() == Some(&diff) {
                    continue;
                }

                info!("📡 State diff received: {diff:?}");

                // Handle blink effect (transient)
                if matches!(diff, AppStateDiff::Blink) {
                    info!("⚡ Triggering LED blink effect for 5 seconds");
                    blink_end_time = Some(std::time::Instant::now() + Duration::from_secs(5));
                }

                // Apply the diff locally (don't send back to channel)
                let old_state = state_manager.current_state().clone();
                state_manager.apply_diff(&diff);
                let new_state = state_manager.current_state().clone();

                // Log state transitions for debugging
                if old_state != new_state {
                    info!("🔄 State transition: {old_state:?} -> {new_state:?}");
                }

                last_diff = Some(diff);

                // Check for talk data updates (non-blocking)
                if let Ok(new_talk_data) = talk_data_receiver.try_recv() {
                    info!("📚 Talk data received: {}", new_talk_data.title);
                    talk_data = Some(new_talk_data);
                }

                // Update display
                info!(
                    "📺 Updating display for state: {:?}, talk_data available: {}",
                    state_manager.current_state(),
                    talk_data.is_some()
                );
                if let Err(error) = display_manager
                    .update_display(state_manager.current_state(), talk_data.as_ref())
                {
                    log::error!("Failed to update display: {error:?}");
                }

                // Update LEDs based on state (with blink override)
                let is_blinking = blink_end_time.is_some_and(|end| std::time::Instant::now() < end);
                update_leds_with_blink(
                    state_manager.current_state(),
                    &mut red,
                    &mut green,
                    &mut blue,
                    is_blinking,
                )?;

                // Handle state-specific actions
                match state_manager.current_state() {
                    AppState::Connected { .. } => {
                        if !api_thread_started {
                            spawn_api_thread(diff_sender.clone(), talk_data_sender.clone(), port)?;
                            state_manager.transition_to(AppState::Loading);
                            api_thread_started = true;
                        }
                    }
                    AppState::Initialized => {
                        if !websocket_thread_started && talk_data.is_some() {
                            spawn_websocket_thread(diff_sender.clone(), port)?;
                            websocket_thread_started = true;
                        }
                    }
                    _ => {} // Other states don't trigger new threads
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check if blink timer expired
                if let Some(end_time) = blink_end_time {
                    if std::time::Instant::now() >= end_time {
                        info!("⚡ Blink effect ended");
                        blink_end_time = None;
                        // Update LEDs to remove blink effect
                        if let Err(error) = update_leds_with_blink(
                            state_manager.current_state(),
                            &mut red,
                            &mut green,
                            &mut blue,
                            false,
                        ) {
                            log::error!("Failed to update LEDs after blink: {error:?}");
                        }
                    } else {
                        // Continue blinking - update LEDs for animation
                        let is_blinking = true;
                        if let Err(error) = update_leds_with_blink(
                            state_manager.current_state(),
                            &mut red,
                            &mut green,
                            &mut blue,
                            is_blinking,
                        ) {
                            log::error!("Failed to update LEDs during blink: {error:?}");
                        }
                    }
                } else {
                    // No blinking active - sleep briefly to reduce CPU usage
                    thread::sleep(Duration::from_millis(10));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("State diff channel disconnected");
                break;
            }
        }
    }

    Ok(())
}

/// Spawn `WiFi` connection thread with proper error handling
fn spawn_wifi_thread(
    diff_sender: mpsc::Sender<AppStateDiff>,
    modem: esp_idf_svc::hal::modem::Modem,
    sysloop: EspSystemEventLoop,
) -> anyhow::Result<()> {
    info!("🔄 Starting WiFi connection thread");
    thread::Builder::new()
        .name("wifi_thread".to_string())
        .stack_size(32 * 1024) // 32KB stack
        .spawn(move || {
            wifi_thread(diff_sender, modem, sysloop);
        })
        .context("Failed to spawn WiFi thread")?;
    Ok(())
}

/// Spawn API loading thread with proper error handling
fn spawn_api_thread(
    diff_sender: mpsc::Sender<AppStateDiff>,
    talk_data_sender: mpsc::Sender<TalkData>,
    port: u16,
) -> anyhow::Result<()> {
    info!("📶 WiFi connected, starting API loading");
    thread::Builder::new()
        .name("api_thread".to_string())
        .stack_size(16 * 1024) // 16KB stack
        .spawn(move || {
            api_thread(diff_sender, talk_data_sender, port);
        })
        .context("Failed to spawn API thread")?;
    Ok(())
}

/// Spawn WebSocket connection thread with proper error handling
fn spawn_websocket_thread(
    diff_sender: mpsc::Sender<AppStateDiff>,
    port: u16,
) -> anyhow::Result<()> {
    info!("🔌 Starting WebSocket connection");
    thread::Builder::new()
        .name("websocket_thread".to_string())
        .stack_size(16 * 1024) // 16KB stack
        .spawn(move || {
            websocket_thread(diff_sender, port);
        })
        .context("Failed to spawn WebSocket thread")?;
    Ok(())
}

/// Update LED state based on application state with optional blink override
fn update_leds_with_blink(
    state: &AppState,
    red: &mut PinDriver<'_, Gpio39, Output>,
    green: &mut PinDriver<'_, Gpio40, Output>,
    blue: &mut PinDriver<'_, Gpio41, Output>,
    is_blinking: bool,
) -> anyhow::Result<()> {
    // If blinking, use yellow blink pattern (regardless of state)
    if is_blinking {
        // Calculate blink state (500ms on/off cycle)
        let blink_on = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            / 500)
            % 2
            == 0;

        if blink_on {
            // Yellow flash during blink (red + green)
            red.set_high()?;
            green.set_high()?;
            blue.set_low()?;
        } else {
            // Off during blink
            red.set_low()?;
            green.set_low()?;
            blue.set_low()?;
        }
        return Ok(());
    }

    // Normal LED state when not blinking
    update_leds_normal(state, red, green, blue)
}

/// Normal LED state without blink effects
fn update_leds_normal(
    state: &AppState,
    red: &mut PinDriver<'_, Gpio39, Output>,
    green: &mut PinDriver<'_, Gpio40, Output>,
    blue: &mut PinDriver<'_, Gpio41, Output>,
) -> anyhow::Result<()> {
    // Turn off all LEDs first
    red.set_low()?;
    green.set_low()?;
    blue.set_low()?;

    match state {
        AppState::Booting => {
            blue.set_high()?; // Blue for booting
        }
        AppState::Connecting { .. } => {
            red.set_high()?; // Red for connecting
        }
        AppState::Connected { .. } => {
            green.set_high()?; // Green for connected
        }
        AppState::Loading => {
            // Yellow (red + green) for loading
            red.set_high()?;
            green.set_high()?;
        }
        AppState::Initialized => {
            blue.set_high()?; // Blue for initialized
        }
        AppState::Play { mode, .. } => {
            match mode {
                StateMode::Paused => {
                    red.set_high()?; // Red for paused
                }
                StateMode::Running => {
                    green.set_high()?; // Green for running
                }
                StateMode::Done => {
                    blue.set_high()?; // Blue for done
                }
            }
        }
        AppState::Error { .. } => {
            red.set_high()?; // Solid red for error
        }
    }

    Ok(())
}

/// `WiFi` connection thread
#[allow(clippy::needless_pass_by_value)] // Need owned values for thread
fn wifi_thread(
    diff_sender: mpsc::Sender<AppStateDiff>,
    modem: esp_idf_svc::hal::modem::Modem,
    sysloop: EspSystemEventLoop,
) {
    info!("📶 WiFi thread started");

    match wifi_sync(WIFI_SSID, WIFI_PASSWORD, modem, sysloop) {
        Ok(wifi) => {
            info!("✅ WiFi connected successfully");
            if let Err(error) = diff_sender.send(AppStateDiff::Transition(AppState::Connected {
                ssid: WIFI_SSID.to_string(),
            })) {
                log::error!("Failed to send Connected diff: {error}");
            }

            // Keep the WiFi connection alive by holding the wifi object
            // This prevents the WiFi driver from being dropped
            let _wifi = wifi; // Move ownership to keep it alive

            // Sleep forever to keep the thread and WiFi connection alive
            loop {
                thread::sleep(Duration::from_secs(3600)); // Sleep for 1 hour at a time
            }
        }
        Err(error) => {
            log::error!("❌ WiFi connection failed: {error:?}");
            if let Err(send_error) = diff_sender.send(AppStateDiff::Error {
                message: format!("WiFi failed: {error}"),
            }) {
                log::error!("Failed to send Error diff: {send_error}");
            }
        }
    }
}

/// API loading thread
#[allow(clippy::needless_pass_by_value)] // Need owned values for thread
fn api_thread(
    diff_sender: mpsc::Sender<AppStateDiff>,
    talk_data_sender: mpsc::Sender<TalkData>,
    port: u16,
) {
    info!("🌐 API thread started");

    let base_url = format!("http://{TOBOGGAN_HOST}:{port}");
    match Api::new(base_url) {
        Ok(mut api) => match api.talk() {
            Ok(talk_data) => {
                info!(
                    "📚 Talk loaded: title='{}', slides: {}",
                    talk_data.title,
                    talk_data.slide_count()
                );

                // Send talk data through dedicated channel
                if let Err(error) = talk_data_sender.send(talk_data) {
                    log::error!("Failed to send talk data: {error}");
                }

                // Send simple initialized state
                if let Err(error) =
                    diff_sender.send(AppStateDiff::Transition(AppState::Initialized))
                {
                    log::error!("Failed to send Initialized diff: {error}");
                }
            }
            Err(error) => {
                log::error!("❌ Failed to load talk: {error:?}");
                if let Err(send_error) = diff_sender.send(AppStateDiff::Error {
                    message: format!("Talk loading failed: {error}"),
                }) {
                    log::error!("Failed to send Error diff: {send_error}");
                }
            }
        },
        Err(error) => {
            log::error!("❌ Failed to create API client: {error:?}");
            if let Err(send_error) = diff_sender.send(AppStateDiff::Error {
                message: format!("API client failed: {error}"),
            }) {
                log::error!("Failed to send Error diff: {send_error}");
            }
        }
    }
}

/// WebSocket connection thread
#[allow(clippy::needless_pass_by_value)] // Need owned values for thread
fn websocket_thread(diff_sender: mpsc::Sender<AppStateDiff>, port: u16) {
    info!("🔌 WebSocket thread started");

    if let Err(err) = connect_to_ws(TOBOGGAN_HOST, port, &diff_sender) {
        error!("Fail to connect to WS: {err:?}");
    }
}
