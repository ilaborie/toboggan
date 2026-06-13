use anyhow::{Context, Result};
use iced::Settings;
use iced::window::icon;
use toboggan_client::TobogganConfig;

mod app;
pub use app::App;

mod constants;
mod icons;
mod message;
mod state;
mod styles;
mod views;
mod widgets;

fn title(_app: &App) -> String {
    String::from("Toboggan Desktop")
}

/// Runs the iced desktop application against a running server.
///
/// Must be called on the main thread (iced/winit owns the macOS main thread) and
/// without a surrounding async runtime. Shared by the standalone
/// `toboggan-desktop` binary and the unified `toboggan desktop` subcommand.
///
/// # Errors
/// Returns an error if the iced runtime fails to start or run.
pub fn run(config: TobogganConfig) -> Result<()> {
    // Setup Lucide icons font
    let lucide_font = lucide_icons::LUCIDE_FONT_BYTES;

    // Run the application with iced 0.14 API
    iced::application(move || App::new(config.clone()), App::update, App::view)
        .title(title)
        .settings(Settings::default())
        .window(iced::window::Settings {
            icon: icon::from_file_data(
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.ico")),
                None,
            )
            .ok(),
            size: iced::Size::new(1280.0, 720.0),
            resizable: true,
            decorations: true,
            ..Default::default()
        })
        .font(lucide_font)
        .subscription(App::subscription)
        .theme(App::theme)
        .run()
        .context("Running application")?;

    Ok(())
}
