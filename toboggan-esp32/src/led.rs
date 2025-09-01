use crate::config::led::{BLINK_INTERVAL_FAST, BLINK_INTERVAL_NORMAL};
use crate::state::{AppState, StateMode};
use anyhow::Context;
use esp_idf_svc::hal::gpio::{AnyIOPin, IOPin, Output, PinDriver};
use log::info;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbColor {
    pub red: bool,
    pub green: bool,
    pub blue: bool,
}

impl RgbColor {
    pub const OFF: Self = Self {
        red: false,
        green: false,
        blue: false,
    };

    pub const RED: Self = Self {
        red: true,
        green: false,
        blue: false,
    };

    pub const GREEN: Self = Self {
        red: false,
        green: true,
        blue: false,
    };

    pub const BLUE: Self = Self {
        red: false,
        green: false,
        blue: true,
    };

    pub const YELLOW: Self = Self {
        red: true,
        green: true,
        blue: false,
    };

    pub const CYAN: Self = Self {
        red: false,
        green: true,
        blue: true,
    };

    pub const WHITE: Self = Self {
        red: true,
        green: true,
        blue: true,
    };

    pub const ORANGE: Self = Self {
        red: true,
        green: false, // Fixed: was incorrectly true (making it yellow)
        blue: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LedPattern {
    Solid(RgbColor),
    Blinking(RgbColor, u64), // color and blink interval in milliseconds
}

impl LedPattern {
    #[must_use]
    pub fn from_state(state: &AppState) -> Self {
        match state {
            AppState::Booting => LedPattern::Blinking(RgbColor::ORANGE, BLINK_INTERVAL_NORMAL),
            AppState::Connecting { .. } => {
                LedPattern::Blinking(RgbColor::YELLOW, BLINK_INTERVAL_NORMAL)
            }
            AppState::Connected { .. } => LedPattern::Solid(RgbColor::GREEN),
            AppState::Loading => LedPattern::Blinking(RgbColor::WHITE, BLINK_INTERVAL_NORMAL),
            AppState::Initialized => LedPattern::Solid(RgbColor::CYAN),
            AppState::Play { mode, .. } => match mode {
                StateMode::Paused => LedPattern::Solid(RgbColor::WHITE),
                StateMode::Running => LedPattern::Solid(RgbColor::BLUE),
                StateMode::Done => LedPattern::Solid(RgbColor::GREEN),
            },
            AppState::Error { .. } => LedPattern::Blinking(RgbColor::RED, BLINK_INTERVAL_FAST),
        }
    }
}

pub struct LedController {
    red_pin: PinDriver<'static, AnyIOPin, Output>,
    green_pin: PinDriver<'static, AnyIOPin, Output>,
    blue_pin: PinDriver<'static, AnyIOPin, Output>,
    current_pattern: LedPattern,
}

impl LedController {
    /// Create a new `LedController` instance
    ///
    /// # Errors
    /// Returns error if GPIO pin initialization fails
    pub fn new(
        red_pin: impl IOPin + 'static,
        green_pin: impl IOPin + 'static,
        blue_pin: impl IOPin + 'static,
    ) -> anyhow::Result<Self> {
        let red_pin = PinDriver::output(red_pin.downgrade()).context("initialize red LED pin")?;
        let green_pin =
            PinDriver::output(green_pin.downgrade()).context("initialize green LED pin")?;
        let blue_pin =
            PinDriver::output(blue_pin.downgrade()).context("initialize blue LED pin")?;

        let mut controller = Self {
            red_pin,
            green_pin,
            blue_pin,
            current_pattern: LedPattern::Solid(RgbColor::OFF),
        };

        controller.turn_off()?;

        Ok(controller)
    }

    /// Set the LED color
    ///
    /// # Errors
    /// Returns error if GPIO pin state changes fail
    pub fn set_color(&mut self, color: RgbColor) -> anyhow::Result<()> {
        if color.red {
            self.red_pin.set_high().context("set red LED high")?;
        } else {
            self.red_pin.set_low().context("set red LED low")?;
        }

        if color.green {
            self.green_pin.set_high().context("set green LED high")?;
        } else {
            self.green_pin.set_low().context("set green LED low")?;
        }

        if color.blue {
            self.blue_pin.set_high().context("set blue LED high")?;
        } else {
            self.blue_pin.set_low().context("set blue LED low")?;
        }

        Ok(())
    }

    /// Turn off all LEDs
    ///
    /// # Errors
    /// Returns error if GPIO pin state changes fail
    pub fn turn_off(&mut self) -> anyhow::Result<()> {
        self.set_color(RgbColor::OFF)
    }

    pub fn set_pattern(&mut self, pattern: LedPattern) {
        if self.current_pattern != pattern {
            info!(
                "LED pattern change: {:?} -> {:?}",
                self.current_pattern, pattern
            );
            self.current_pattern = pattern;
        }
    }
}
