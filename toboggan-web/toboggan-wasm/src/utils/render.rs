use crate::{ConnectionStatus, StateClassMapper};
use toboggan_core::State;

/// Maps State enum to CSS class names
impl StateClassMapper<State> for State {
    fn to_css_class(&self) -> &'static str {
        match self {
            State::Init => "init",
            State::Paused { .. } => "paused",
            State::Running { .. } => "running",
            State::Done { .. } => "done",
        }
    }
}

/// Maps `ConnectionStatus` enum to CSS class names
impl StateClassMapper<ConnectionStatus> for ConnectionStatus {
    fn to_css_class(&self) -> &'static str {
        match self {
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Closed => "closed",
            ConnectionStatus::Reconnecting { .. } => "reconnecting",
            ConnectionStatus::Error { .. } => "error",
        }
    }
}

/// Format slide information as "current/total" string
#[must_use]
pub fn format_slide_info(current: Option<u8>, total: Option<usize>) -> String {
    let current_str = current.map_or_else(|| "-".to_string(), |current| (current + 1).to_string());
    let total_str = total.map_or_else(|| "-".to_string(), |total| total.to_string());
    format!("{current_str}/{total_str}")
}

/// Format duration as HH:MM:SS string
#[must_use]
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}
