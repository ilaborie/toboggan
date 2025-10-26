use toboggan_client::ConnectionStatus as CoreConnectionStatus;

use crate::State;

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting,
    Error,
}

impl From<CoreConnectionStatus> for ConnectionStatus {
    fn from(value: CoreConnectionStatus) -> Self {
        match value {
            CoreConnectionStatus::Connecting => Self::Connecting,
            CoreConnectionStatus::Connected => Self::Connected,
            CoreConnectionStatus::Closed => Self::Closed,
            CoreConnectionStatus::Reconnecting { .. } => Self::Reconnecting,
            CoreConnectionStatus::Error { .. } => Self::Error,
        }
    }
}

#[uniffi::export(with_foreign)]
pub trait ClientNotificationHandler: Send + Sync {
    fn on_state_change(&self, state: State);
    fn on_talk_change(&self, state: State);
    fn on_connection_status_change(&self, status: ConnectionStatus);
    fn on_error(&self, error: String);
}
