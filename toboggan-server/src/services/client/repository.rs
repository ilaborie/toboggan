use std::net::IpAddr;
use std::sync::Arc;

use slotmap::SlotMap;
use toboggan_core::{ClientId, ClientInfo, ClientsResponse, Notification, Timestamp};
use tokio::sync::{RwLock, watch};
use tracing::debug;

/// Entry stored in the client repository
pub(super) struct ClientEntry {
    pub info: ClientInfo,
    pub sender: watch::Sender<Notification>,
}

/// Repository for client storage using `SlotMap`
#[derive(Clone)]
pub(super) struct ClientRepository {
    clients: Arc<RwLock<SlotMap<slotmap::DefaultKey, ClientEntry>>>,
    max_clients: usize,
}

impl ClientRepository {
    /// Creates a new `ClientRepository` with the given maximum capacity
    #[must_use]
    pub(super) fn new(max_clients: usize) -> Self {
        Self {
            clients: Arc::new(RwLock::new(SlotMap::new())),
            max_clients,
        }
    }

    /// Inserts a new client entry and returns its `ClientId`
    ///
    /// Returns `None` if the repository is at maximum capacity
    pub(super) async fn insert(
        &self,
        name: String,
        ip_addr: IpAddr,
        connected_at: Timestamp,
        sender: watch::Sender<Notification>,
    ) -> Option<ClientId> {
        let mut clients = self.clients.write().await;

        if clients.len() >= self.max_clients {
            return None;
        }

        // Use insert_with_key to get the correct ID for ClientInfo
        let key = clients.insert_with_key(|key| {
            let id = ClientId::from_key(key);
            let info = ClientInfo {
                id,
                name,
                ip_addr,
                connected_at,
            };
            ClientEntry { info, sender }
        });
        Some(ClientId::from_key(key))
    }

    /// Removes a client by ID and returns the entry if it existed
    pub(super) async fn remove(&self, id: ClientId) -> Option<ClientEntry> {
        let mut clients = self.clients.write().await;
        clients.remove(id.key())
    }

    /// Returns true if the repository has capacity for more clients
    pub(super) async fn has_capacity(&self) -> bool {
        let clients = self.clients.read().await;
        clients.len() < self.max_clients
    }

    /// Returns the number of connected clients
    pub(super) async fn len(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }

    /// Removes disconnected clients and returns the number removed
    pub(super) async fn cleanup_disconnected(&self) -> usize {
        let mut clients = self.clients.write().await;
        let initial_count = clients.len();

        clients.retain(|key, entry| {
            let is_connected = !entry.sender.is_closed();
            if !is_connected {
                debug!(
                    client_id = ?ClientId::from_key(key),
                    name = %entry.info.name,
                    "Removing disconnected client"
                );
            }
            is_connected
        });

        initial_count - clients.len()
    }

    /// Returns a response with all connected clients
    pub(super) async fn connected_clients(&self) -> ClientsResponse {
        let clients = self.clients.read().await;
        let clients_vec = clients.values().map(|entry| entry.info.clone()).collect();
        ClientsResponse {
            clients: clients_vec,
        }
    }

    /// Sends a notification to all connected clients
    pub(super) async fn notify_all(&self, notification: &Notification) {
        let clients = self.clients.read().await;
        for (key, entry) in clients.iter() {
            if entry.sender.send(notification.clone()).is_err() {
                tracing::warn!(
                    client_id = ?ClientId::from_key(key),
                    "Failed to send notification to client, client may have disconnected"
                );
            }
        }
    }

    /// Sends a notification to all clients except the excluded one
    pub(super) async fn notify_others(&self, exclude: ClientId, notification: &Notification) {
        let clients = self.clients.read().await;
        for (key, entry) in clients.iter() {
            if key == exclude.key() {
                continue;
            }
            if entry.sender.send(notification.clone()).is_err() {
                tracing::warn!(
                    client_id = ?ClientId::from_key(key),
                    "Failed to send notification to client, client may have disconnected"
                );
            }
        }
    }
}
