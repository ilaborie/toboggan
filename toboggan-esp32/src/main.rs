use anyhow::Context;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals};

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let sysloop = EspSystemEventLoop::take()?;

    toboggan_esp32::run(peripherals, sysloop)
}
