use anyhow::{Context, Result};
use iced::Settings;
use toboggan_client::TobogganConfig;
use toboggan_desktop::App;

fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Setup Lucide icons font
    let lucide_font = lucide_icons::LUCIDE_FONT_BYTES;

    let config = TobogganConfig::default();

    // Run the application
    iced::application("Toboggan Desktop", App::update, App::view)
        .settings(Settings::default())
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 720.0),
            resizable: true,
            decorations: true,
            ..Default::default()
        })
        .font(lucide_font)
        .subscription(App::subscription)
        .theme(App::theme)
        .run_with(move || App::new(config))
        .context("Running application")?;

    Ok(())
}
