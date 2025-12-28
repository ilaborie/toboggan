use std::net::IpAddr;
use std::time::Duration;

use toboggan_core::{ClientId, ClientsResponse, Notification, Timestamp};
use tokio::sync::watch;
use tracing::info;

mod repository;
use self::repository::ClientRepository;
use crate::ApiError;

/// Service for managing client life cycle and notifications
#[derive(Clone)]
pub struct ClientService {
    repository: ClientRepository,
}

impl ClientService {
    /// Creates a new `ClientService` with the given maximum client capacity
    #[must_use]
    pub fn new(max_clients: usize) -> Self {
        Self {
            repository: ClientRepository::new(max_clients),
        }
    }

    /// Registers a new client for notifications
    ///
    /// # Errors
    /// Returns an error if the maximum number of clients is exceeded
    pub async fn register_client(
        &self,
        name: String,
        ip_addr: IpAddr,
        initial_notification: Notification,
    ) -> Result<(ClientId, watch::Receiver<Notification>), ApiError> {
        // Cleanup before checking capacity
        self.repository.cleanup_disconnected().await;

        if !self.repository.has_capacity().await {
            return Err(ApiError::TooManyClients);
        }

        let (tx, rx) = watch::channel(initial_notification);
        let connected_at = Timestamp::now();

        let Some(client_id) = self
            .repository
            .insert(name.clone(), ip_addr, connected_at, tx)
            .await
        else {
            return Err(ApiError::TooManyClients);
        };

        // Broadcast ClientConnected to other clients
        let connect_notification = Notification::client_connected(client_id, &name);
        self.repository
            .notify_others(client_id, &connect_notification)
            .await;

        let active_clients = self.repository.len().await;
        info!(
            ?client_id,
            %name,
            %ip_addr,
            active_clients,
            "Client registered"
        );

        Ok((client_id, rx))
    }

    /// Unregisters a client and broadcasts disconnection to others
    pub async fn unregister_client(&self, client_id: ClientId) {
        if let Some(entry) = self.repository.remove(client_id).await {
            // Broadcast ClientDisconnected to remaining clients
            let disconnect_notification =
                Notification::client_disconnected(client_id, &entry.info.name);
            self.repository.notify_all(&disconnect_notification).await;

            let active_clients = self.repository.len().await;
            info!(
                ?client_id,
                name = %entry.info.name,
                active_clients,
                "Client unregistered"
            );
        }
    }

    /// Background task to periodically cleanup disconnected clients
    pub async fn cleanup_clients_task(&self, cleanup_interval: Duration) {
        let mut interval = tokio::time::interval(cleanup_interval);

        loop {
            interval.tick().await;
            let removed = self.repository.cleanup_disconnected().await;
            if removed > 0 {
                let active_clients = self.repository.len().await;
                info!(
                    removed_count = removed,
                    active_clients, "Cleaned up disconnected clients"
                );
            }
        }
    }

    /// Sends a notification to all connected clients
    pub async fn notify_all(&self, notification: &Notification) {
        self.repository.notify_all(notification).await;
    }

    /// Sends a notification to all clients except the excluded one
    pub async fn notify_others(&self, exclude: ClientId, notification: &Notification) {
        self.repository.notify_others(exclude, notification).await;
    }

    /// Returns the list of connected clients
    pub async fn connected_clients(&self) -> ClientsResponse {
        self.repository.connected_clients().await
    }

    /// Returns the number of active clients
    pub async fn active_clients_count(&self) -> usize {
        self.repository.len().await
    }
}
