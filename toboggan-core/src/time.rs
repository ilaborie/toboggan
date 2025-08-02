//! Time-related wrapper types and utility functions.
//!
//! This module provides wrapper types around jiff time types with
//! additional utility methods and trait implementations.

use core::fmt::{self, Display, Formatter};
use core::time::Duration;

use serde::{Deserialize, Serialize};

/// Wrapper around `jiff::Timestamp` with additional utility methods.
///
/// This type provides a convenient wrapper for timestamp operations
/// while maintaining compatibility with jiff's timestamp functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub jiff::Timestamp);

impl Timestamp {
    /// Get the current timestamp
    #[must_use]
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }

    /// Get elapsed duration since this timestamp
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        let signed_duration = jiff::Timestamp::now().duration_since(self.0);
        TryInto::<Duration>::try_into(signed_duration).unwrap_or(Duration::ZERO)
    }
}

impl Display for Timestamp {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, fmt)
    }
}

/// Wrapper around `jiff::civil::Date` with additional utility methods.
///
/// This type provides a convenient wrapper for date operations
/// while maintaining compatibility with jiff's date functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date(pub jiff::civil::Date);

impl Date {
    /// Create a new Date from year, month, and day.
    ///
    /// # Errors
    ///
    /// Returns an error if the date is invalid (e.g., month out of range, day out of range for the month).
    pub fn new(year: i16, month: i8, day: i8) -> Result<Self, jiff::Error> {
        jiff::civil::Date::new(year, month, day).map(Self)
    }

    /// Get today's date
    #[must_use]
    pub fn today() -> Self {
        let now = jiff::Zoned::now();
        Self(now.date())
    }

    /// Create a new Date from year, month, and day.
    ///
    /// # Panics
    ///
    /// Panics if the date is invalid (e.g., month out of range, day out of range for the month).
    #[allow(clippy::expect_used)] // Acceptable for date validation - invalid dates should panic
    #[must_use]
    pub fn ymd(year: i16, month: i8, day: i8) -> Date {
        Date::new(year, month, day).expect("valid date")
    }
}

impl Display for Date {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, fmt)
    }
}
