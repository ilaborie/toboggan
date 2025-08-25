use anyhow::{Context, Result};
use iced::Settings;
use toboggan_desktop::App;

fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Setup Lucide icons font
    let lucide_font = lucide_icons::lucide_font_bytes();

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
        .run_with(App::new)
        .context("Running application")?;

    Ok(())
}
