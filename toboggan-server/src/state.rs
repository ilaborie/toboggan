use axum::extract::FromRef;
use toboggan_core::{Command, Notification, Talk, Timestamp};

use crate::services::{ClientService, TalkService};
use crate::{HealthResponse, HealthResponseStatus};

impl FromRef<TobogganState> for TalkService {
    fn from_ref(state: &TobogganState) -> Self {
        state.talk_service.clone()
    }
}

impl FromRef<TobogganState> for ClientService {
    fn from_ref(state: &TobogganState) -> Self {
        state.client_service.clone()
    }
}

/// Thin coordinator/facade that orchestrates `TalkService` and `ClientService`
#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk_service: TalkService,
    client_service: ClientService,
}

impl TobogganState {
    /// Creates a new `TobogganState` with the given services
    #[must_use]
    pub fn new(talk_service: TalkService, client_service: ClientService) -> Self {
        Self {
            started_at: Timestamp::now(),
            talk_service,
            client_service,
        }
    }

    /// Returns the health status of the server
    pub(crate) async fn health(&self) -> HealthResponse {
        let status = HealthResponseStatus::Ok;
        let started_at = self.started_at;
        let elapsed = started_at.elapsed();
        let talk = self.talk_service.title().await;
        let active_clients = self.client_service.active_clients_count().await;

        HealthResponse {
            status,
            started_at,
            elapsed,
            talk,
            active_clients,
        }
    }

    /// Handles a command, broadcasts the notification to all clients, and returns it
    pub async fn handle_command(&self, command: &Command) -> Notification {
        let start_time = std::time::Instant::now();

        let notification = self.talk_service.handle_command(command).await;
        self.client_service.notify_all(&notification).await;

        let active_clients = self.client_service.active_clients_count().await;
        tracing::debug!(
            ?command,
            duration_ms = start_time.elapsed().as_millis(),
            active_clients,
            "Command handled and broadcast completed"
        );

        notification
    }

    /// Reloads the talk and broadcasts the change to all clients
    ///
    /// # Errors
    /// Returns an error if the new talk has no slides
    pub async fn reload_talk(&self, new_talk: Talk) -> anyhow::Result<()> {
        let notification = self.talk_service.reload_talk(new_talk).await?;
        self.client_service.notify_all(&notification).await;
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
