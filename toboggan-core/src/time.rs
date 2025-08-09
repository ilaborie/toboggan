//! Time-related wrapper types and utility functions.
//!
//! This module provides wrapper types around jiff time types with
//! additional utility methods and trait implementations.

use core::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    derive_more::Add,
)]
pub struct Duration(core::time::Duration);

impl Duration {
    pub const ZERO: Self = Self(core::time::Duration::ZERO);

    #[must_use]
    pub fn from_secs(secs: u64) -> Self {
        Self(core::time::Duration::from_secs(secs))
    }

    #[must_use]
    pub fn from_millis(millis: u64) -> Self {
        Self(core::time::Duration::from_millis(millis))
    }
}

impl From<Duration> for core::time::Duration {
    fn from(value: Duration) -> Self {
        value.0
    }
}

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
        let duration = TryInto::<core::time::Duration>::try_into(signed_duration)
            .unwrap_or(core::time::Duration::ZERO);
        Duration(duration)
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

#[cfg(feature = "openapi")]
mod openapi {
    use std::borrow::Cow;

    use utoipa::openapi::schema::Schema;
    use utoipa::openapi::{KnownFormat, ObjectBuilder, RefOr, SchemaFormat, Type};
    use utoipa::{PartialSchema, ToSchema};

    use super::{Date, Duration, Timestamp};

    impl ToSchema for Duration {
        fn name() -> Cow<'static, str> {
            Cow::Borrowed("Duration")
        }
    }

    impl PartialSchema for Duration {
        fn schema() -> RefOr<Schema> {
            RefOr::T(Schema::Object(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .property(
                        "secs",
                        RefOr::T(Schema::Object(
                            ObjectBuilder::new()
                                .schema_type(Type::Number)
                                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                                .build(),
                        )),
                    )
                    .property(
                        "nanos",
                        RefOr::T(Schema::Object(
                            ObjectBuilder::new()
                                .schema_type(Type::Number)
                                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
                                .build(),
                        )),
                    )
                    .build(),
            ))
        }
    }

    impl ToSchema for Timestamp {
        fn name() -> Cow<'static, str> {
            Cow::Borrowed("Timestamp")
        }
    }

    impl PartialSchema for Timestamp {
        fn schema() -> RefOr<Schema> {
            RefOr::T(Schema::Object(
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime)))
                    .build(),
            ))
        }
    }

    impl ToSchema for Date {
        fn name() -> Cow<'static, str> {
            Cow::Borrowed("Date")
        }
    }

    impl PartialSchema for Date {
        fn schema() -> RefOr<Schema> {
            RefOr::T(Schema::Object(
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(SchemaFormat::KnownFormat(KnownFormat::Date)))
                    .build(),
            ))
        }
    }
}
