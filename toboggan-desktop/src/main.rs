use anyhow::{Context, Result};
use iced::Settings;
use iced::window::icon;
use toboggan_client::TobogganConfig;
use toboggan_desktop::App;

fn title(_app: &App) -> String {
    String::from("Toboggan Desktop")
}

fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt::init();

    // Setup Lucide icons font
    let lucide_font = lucide_icons::LUCIDE_FONT_BYTES;

    let config = TobogganConfig::default();

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
