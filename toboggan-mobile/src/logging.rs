//! Getting the client's own diagnostics onto the phone.
//!
//! Every `tracing` line in [`toboggan_client`] was written into a subscriber
//! that no host app ever installed, so on a device they all went nowhere. That
//! left the in-app log sheet — built precisely because someone debugging a phone
//! that will not reach the server is standing in a room, not sitting at a
//! debugger — showing only the lines the app wrote about itself, and none of the
//! ones about the connection it is failing to make.
//!
//! The host installs a [`LogSink`] once at startup and the two logs become one.

use std::fmt::Write as _;
use std::sync::Arc;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt as _};
use tracing_subscriber::util::SubscriberInitExt as _;

/// How serious a log line is. Mirrors `tracing`'s levels, minus `TRACE`, which
/// no client emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Where the client's log lines go on the host side.
///
/// Implemented by the app — on iOS by the same `AppLog` the log sheet renders,
/// so the Rust and Swift halves of a failed connection appear in one list, in
/// order. An implementation must not log through `tracing` itself.
#[uniffi::export(with_foreign)]
pub trait LogSink: Send + Sync {
    fn log(&self, level: LogLevel, target: String, message: String);
}

/// Installs `sink` as the destination for this crate's log lines.
///
/// Safe to call more than once: the second call is ignored rather than
/// replacing the first, because a global subscriber can only be set once and
/// failing here would take the host app down over a log.
#[uniffi::export]
pub fn init_logging(sink: Arc<dyn LogSink>, verbose: bool) {
    let level = if verbose {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let _ = tracing_subscriber::registry()
        .with(SinkLayer { sink }.with_filter(level))
        .try_init();
}

struct SinkLayer {
    sink: Arc<dyn LogSink>,
}

impl<S: Subscriber> Layer<S> for SinkLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::ERROR => LogLevel::Error,
            Level::WARN => LogLevel::Warn,
            Level::INFO => LogLevel::Info,
            // The filter above admits nothing below DEBUG, so TRACE cannot
            // arrive; it reads as DEBUG rather than adding a variant the host
            // would have to render.
            Level::DEBUG | Level::TRACE => LogLevel::Debug,
        };

        let mut message = MessageVisitor::default();
        event.record(&mut message);
        self.sink
            .log(level, metadata.target().to_owned(), message.0);
    }
}

/// Flattens an event's fields into one line.
///
/// `message` is the line itself and the rest are `key=value` context, which is
/// the shape the app's log sheet already displays.
#[derive(Default)]
struct MessageVisitor(String);

impl MessageVisitor {
    fn push(&mut self, field: &Field, value: &dyn std::fmt::Display) {
        // Writing to a String cannot fail; the result is discarded rather than
        // unwrapped so that a log line can never abort the host app.
        if field.name() == "message" {
            let _ = write!(self.0, "{value}");
        } else {
            let separator = if self.0.is_empty() { "" } else { " " };
            let _ = write!(self.0, "{separator}{}={value}", field.name());
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.push(field, &format_args!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, &value);
    }
}
