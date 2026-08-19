use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

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
    derive_more::Add,
    derive_more::Deref,
    derive_more::From,
)]
/// A span of time, serialized in a form a human can write.
///
/// Wraps [`std::time::Duration`] so that it round-trips through TOML as
/// `"2m 30s"` rather than a struct of seconds and nanoseconds — a slide's
/// `duration` front matter is written by hand.
pub struct Duration(std::time::Duration);

impl Duration {
    /// No time at all.
    pub const ZERO: Self = Self(std::time::Duration::ZERO);

    /// A duration of whole seconds.
    #[must_use]
    pub fn from_secs(secs: u64) -> Self {
        Self(std::time::Duration::from_secs(secs))
    }

    /// A duration of whole milliseconds.
    #[must_use]
    pub fn from_millis(millis: u64) -> Self {
        Self(std::time::Duration::from_millis(millis))
    }
}

impl From<Duration> for std::time::Duration {
    fn from(value: Duration) -> Self {
        value.0
    }
}

impl Display for Duration {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let secs = self.0.as_secs();
        let mins = secs / 60;
        let secs = secs - (60 * mins);
        write!(fmt, "{mins:02}:{secs:02}")
    }
}

/// Serde support for [`Duration`], in humantime form.
///
/// A duration is written by a human in front matter and read back by one in a
/// built artifact, so it travels as `"2m 30s"` rather than as a number.
pub mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::Duration;

    impl Serialize for Duration {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            humantime::format_duration(self.0)
                .to_string()
                .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Duration {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let duration = String::deserialize(deserializer)?;
            humantime::parse_duration(&duration)
                .map(Duration)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// An instant, used for "when did this client connect".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub jiff::Timestamp);

impl Timestamp {
    /// The current instant.
    #[must_use]
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }

    /// How long ago this was. Saturates at zero for an instant in the future.
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

/// A calendar date, with no time and no zone: the day a talk is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date(jiff::civil::Date);

impl Date {
    /// A date from its parts.
    ///
    /// # Errors
    /// Returns an error if the parts do not name a real date.
    pub fn new(year: i16, month: i8, day: i8) -> Result<Self, jiff::Error> {
        jiff::civil::Date::new(year, month, day).map(Self)
    }

    /// Today, in the local time zone.
    #[must_use]
    pub fn today() -> Self {
        let now = jiff::Zoned::now();
        Self(now.date())
    }

    /// A date from its parts, falling back to today and logging a warning.
    ///
    /// For the places that would rather show a deck with the wrong date than
    /// fail to build it at all.
    #[cfg(feature = "tracing")]
    #[must_use]
    pub fn ymd(year: i16, month: i8, day: i8) -> Date {
        Date::new(year, month, day).unwrap_or_else(|error| {
            tracing::warn!(?error, year, month, day, "fail to build date");
            Self::today()
        })
    }
}

impl Display for Date {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, fmt)
    }
}

impl FromStr for Date {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try ISO format first (YYYY-MM-DD)
        jiff::civil::Date::strptime("%Y-%m-%d", s)
            .or_else(|err| {
                // For single-digit months/days, parse manually
                let mut parts = s.splitn(3, '-');
                let year: i16 = parts
                    .next()
                    .and_then(|part| part.parse().ok())
                    .ok_or_else(|| err.clone())?;
                let month: i8 = parts
                    .next()
                    .and_then(|part| part.parse().ok())
                    .ok_or_else(|| err.clone())?;
                let day: i8 = parts
                    .next()
                    .and_then(|part| part.parse().ok())
                    .ok_or_else(|| err.clone())?;
                jiff::civil::Date::new(year, month, day)
            })
            .map(Self)
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
