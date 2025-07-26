use core::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(jiff::Timestamp);

impl Timestamp {
    #[must_use]
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        let signed_duration = jiff::Timestamp::now().duration_since(self.0);
        TryInto::<Duration>::try_into(signed_duration).unwrap_or(Duration::ZERO)
    }

    #[must_use]
    pub fn as_millisecond(&self) -> i64 {
        self.0.as_millisecond()
    }

    #[must_use]
    pub fn duration_since(&self, other: Self) -> jiff::SignedDuration {
        self.0.duration_since(other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date(jiff::civil::Date);

impl Date {
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
    #[must_use]
    #[allow(clippy::expect_used)] // Acceptable for date validation - invalid dates should panic
    pub fn new(year: i16, month: i8, day: i8) -> Self {
        let date = jiff::civil::Date::new(year, month, day).expect("valid date");
        Self(date)
    }
}
