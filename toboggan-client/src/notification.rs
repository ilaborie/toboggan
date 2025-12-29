use toboggan_core::{ClientId, State};

use crate::ConnectionStatus;

/// Trait for handling notifications from the Toboggan server.
///
/// Implementations receive callbacks when server events occur.
/// This trait is designed to be implemented by client applications
/// to react to server-side state changes and connection events.
pub trait NotificationHandler: Send + Sync {
    /// Called when the connection status changes (connecting, connected, closed, etc.)
    fn on_connection_status_change(&self, status: ConnectionStatus);

    /// Called when the presentation state changes (slide/step navigation)
    fn on_state_change(&self, state: State);

    /// Called when the talk metadata changes (requires refetch of talk and slides)
    fn on_talk_change(&self, state: State);

    /// Called when an error occurs
    fn on_error(&self, error: String);

    /// Called when this client is registered with the server
    fn on_registered(&self, client_id: ClientId);

    /// Called when another client connects to the server
    fn on_client_connected(&self, client_id: ClientId, name: String);

    /// Called when another client disconnects from the server
    fn on_client_disconnected(&self, client_id: ClientId, name: String);
}
