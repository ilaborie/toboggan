use std::fmt::Debug;

use anyhow::Context;
use esp_idf_svc::hal::{
    delay::Ets,
    gpio::{AnyInputPin, Gpio4, Gpio48, Gpio5, Gpio6, Gpio7, PinDriver},
    spi::{
        config::{Config, MODE_0},
        SpiDeviceDriver, SpiDriverConfig, SPI2,
    },
    units::MegaHertz,
};
use log::info;
use mipidsi::{
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{ColorOrder, Orientation},
    Builder,
};

use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};

const WIDTH: u16 = 320;
const HEIGHT: u16 = 240;

// ## Display
// | Parameter | Value |
// |-----------|-------|
// | **Display Type** | 2.4-inch LCD |
// | **Display Resolution** | 240 x 320 pixels |
// | **Display Interface** | SPI |
// | **Interface Speed** | 40 MHz |
// | **Touch Type** | Capacitive |
// | **Touch Points** | 10 |
// | **Driver IC** | ILI9342C |

// https://docs.espressif.com/projects/esp-idf/en/release-v5.5/esp32s3/api-reference/peripherals/gpio.html

/// Initialize the display with SPI interface
///
/// # Errors
/// Returns error if SPI device creation fails, GPIO pin configuration fails,
/// or display initialization fails
pub fn display(
    spi2: SPI2,
    sclk: Gpio7,
    sdo: Gpio6, // aka miso
    cs: Gpio5,
    dc: Gpio4,
    reset: Gpio48,
    // blacklight: Gpio47,
    buffer: &mut [u8],
) -> anyhow::Result<impl DrawTarget<Color = Rgb565, Error = impl Debug> + use<'_>> {
    let config = Config {
        baudrate: MegaHertz::from(40).into(),
        data_mode: MODE_0,
        ..Default::default()
    };
    // let config = Config::default();

    let bus_config = SpiDriverConfig::new();

    let spi_device = SpiDeviceDriver::new_single::<SPI2>(
        spi2,
        sclk,                // sclk: serial clock
        sdo,                 // sdo: serial data output / MISO
        None::<AnyInputPin>, //Some(mosi), // sdi: serial data input / MOSI
        // None::<AnyOutputPin>, //Some(cs),   // cs: chip select
        Some(cs),
        &bus_config,
        &config,
    )
    .context("create SPI device")?;

    // Define the Data/Command select pin as a digital output (pins.gpio7)
    // LCD interface: use GPIO4 for DC.
    let dc = PinDriver::output(dc).context("data/command pin")?;

    // Display interface
    let di = SpiInterface::new(spi_device, dc, buffer);

    // Reset (GPIO48)
    let rst = PinDriver::output_od(reset).context("reset pin")?;

    // Delay
    // let mut delay = Delay::new_default();
    let mut delay = Ets;

    // Build display
    let display = Builder::new(ILI9342CRgb565, di)
        .reset_pin(rst)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::new().flip_horizontal().flip_vertical())
        .display_size(WIDTH, HEIGHT)
        .init(&mut delay)
        .map_err(|err| anyhow::anyhow!("init display: {err:#?}"))?;

    info!("Display initialized");

    // // Color
    // display
    //     .clear(Rgb565::RED)
    //     .map_err(|err| anyhow!("turn red: {err:?}"))?;

    // info!("blacklight...");

    // let mut backlight = PinDriver::output(blacklight).context("backlight")?;
    // backlight.set_high().context("activate backlight")?;

    // Text
    // Text::new(
    //     "Plop...",
    //     Point::new(80, 110),
    //     MonoTextStyle::new(&FONT_8X13, RgbColor::BLACK),
    // )
    // .draw(&mut display)
    // .context("draw text")?;

    Ok(display)
}

// pub fn display(peripherals: &Peripherals, _text: &str) -> anyhow::Result<()> {
// --- DMA Buffers for SPI ---
// let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(8912);
// let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
// let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

// --- Initialize SPI.
// let spi = Spi::<Blocking>::new(
//     peripherals.spi2,
//     esp_hal::spi::master::Config::default()
//         .with_frequency(Rate::from_mhz(40))
//         .with_mode(esp_hal::spi::Mode::_0),
// )
// .unwrap()
// .with_sck(peripherals.pins.gpio7)
// .with_mosi(peripherals.pins.gpio6)
// .with_dma(peripherals.DMA_CH0)
// .with_buffers(dma_rx_buf, dma_tx_buf);

// let cs_output = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
// let spi_delay = Delay::new_default();
// let spi_device = ExclusiveDevice::new(spi, cs_output, spi_delay).unwrap();

// LCD interface: use GPIO4 for DC.
// let lcd_dc = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
// let buffer: &'static mut [u8; 512] = Box::leak(Box::new([0_u8; 512]));
// let di = SpiInterface::new(spi_device, lcd_dc, buffer);

// let mut display_delay = Delay::new(500u32);

// Reset pin for display: GPIO48 (OpenDrain required).
// let reset = Output::new(
//     peripherals.GPIO48,
//     Level::High,
//     OutputConfig::default().with_drive_mode(DriveMode::OpenDrain),
// );

// let mut display = Builder::new(ILI9486Rgb565, di)
//     .reset_pin(reset)
//     .display_size(320, 240)
//     .color_order(ColorOrder::Bgr)
//     .init(&mut display_delay)
//     .context("creat display")?;

// display.clear(Rgb565::BLUE).context("set blue")?;

// gpio45
// let mut backlight = Output::new(peripherals.pins.gpio47, Level::Low, OutputConfig::default());
// backlight.set_high();

//     info!("Display initialized");

//     Ok(())
// }

/*
=== https://github.dev/georgik/esp32-rust-multi-target-template ===
||| see esp32s3_box


    let mosi = io.pins.gpio6;

    let spi = spi::Spi::new_no_cs_no_miso(
        peripherals.SPI2,
        sclk,
        mosi,
        60u32.MHz(),
        spi::SpiMode::Mode0,
        &mut system.peripheral_clock_control,
        &clocks,
    );

    let mut backlight = io.pins.gpio45.into_push_pull_output();
    let reset = io.pins.gpio48.into_push_pull_output();

    let mut display = mipidsi::Builder::ili9342c_rgb565(di)
        .with_display_size(320, 240)
        .with_orientation(mipidsi::Orientation::PortraitInverted(false))
        .with_color_order(mipidsi::ColorOrder::Bgr)
        .init(&mut delay, Some(reset))
        .unwrap();

*/

// https://github.com/almindor/mipidsi/blob/d85192a933623d6c069f22d5738e25c368f55808/examples/spi-st7789-rpi-zero-w/src/main.rs
// pub fn display(peripherals: &Peripherals, _text: &str) -> anyhow::Result<()> {
//     let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, 60_000_000_u32, Mode::Mode0).unwrap();
//     let spi_device = ExclusiveDevice::new_no_delay(spi, NoCs).unwrap();
//     let mut buffer = [0_u8; 512];
//     let di = SpiInterface::new(spi_device, dc, &mut buffer);
//     let mut delay = Delay::default();

//     let mut display = Builder::new(ST7789, di)
//         .display_size(W as u16, H as u16)
//         .invert_colors(ColorInversion::Inverted)
//         .init(&mut delay)
//         .unwrap();

//     // Text
//     let char_w = 10;
//     let char_h = 20;
//     let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
//     let text = "Hello World ^_^;";
//     let mut text_x = W;
//     let mut text_y = H / 2;

//     // Alternating color
//     let colors = [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE];

//     // Clear the display initially
//     display.clear(colors[0]).unwrap();

//     // Turn on backlight
//     backlight.set_high();

//     Ok(())
// }

// fn draw_smiley<T>(display: &mut T) -> Result<(), T::Error>
// where
//     T: DrawTarget<Color = Rgb565>,
// {
//     // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
//     Circle::new(Point::new(50, 100), 40)
//         .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
//         .draw(display)?;

//     // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
//     Circle::new(Point::new(50, 200), 40)
//         .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
//         .draw(display)?;

//     // Draw an upside down red triangle to represent a smiling mouth
//     Triangle::new(
//         Point::new(130, 140),
//         Point::new(130, 200),
//         Point::new(160, 170),
//     )
//     .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
//     .draw(display)?;

//     // Cover the top part of the mouth with a black triangle so it looks closed instead of open
//     Triangle::new(
//         Point::new(130, 150),
//         Point::new(130, 190),
//         Point::new(150, 170),
//     )
//     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
//     .draw(display)?;

//     Ok(())
// }

// https://github.com/georgik/esp-display-interface-spi-dma/tree/main
// pub fn display(text: &str) -> anyhow::Result<()> {
//     use esp_display_interface_spi_dma::display_interface_spi_dma;

//     let spi = Spi::new_with_config(
//         peripherals.SPI2,
//         esp_hal::spi::master::Config {
//             frequency: 40u32.MHz(),
//             ..esp_hal::spi::master::Config::default()
//         },
//     )
//     .with_sck(lcd_sclk)
//     .with_mosi(lcd_mosi)
//     .with_miso(lcd_miso)
//     .with_cs(lcd_cs)
//     .with_dma(dma_channel.configure(false, DmaPriority::Priority0));

//     let di = display_interface_spi_dma::new_no_cs(LCD_MEMORY_SIZE, spi, lcd_dc);

//     let mut display = mipidsi::Builder::new(mipidsi::models::ILI9341Rgb565, di)
//         .display_size(240, 320)
//         .orientation(mipidsi::options::Orientation::new())
//         .color_order(mipidsi::options::ColorOrder::Bgr)
//         .reset_pin(lcd_reset)
//         .init(&mut delay)
//         .unwrap();

//     let _ = lcd_backlight.set_high();

//     MON!("Initializing...");
//     Text::new(
//         "Initializing...",
//         Point::new(80, 110),
//         MonoTextStyle::new(&FONT_8X13, RgbColor::WHITE),
//     )
//     .draw(&mut display)
//     .unwrap();

//     Ok(())
// }
