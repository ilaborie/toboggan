use std::time::Duration;

/// Re-exported so a caller can tell one refusal from another without taking on
/// a `reqwest` dependency of its own just to name a status code.
pub use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use toboggan_core::{
    ClientInfo, ClientsResponse, Command, Notification, Secret, Slide, SlideId, SlidesResponse,
    TalkResponse,
};

/// Why a call to the server did not produce an answer.
///
/// Three genuinely different failures, kept apart. They used to be one variant
/// wrapping [`reqwest::Error`], which is how a deserialization bug in
/// [`TobogganApi::clients`] went unnoticed: the decode failure reached Python
/// as a `ConnectionError`, sending anyone who saw it hunting for a network
/// problem that was not there. Collapsing them again would hide the next one
/// the same way.
#[derive(Debug, derive_more::Error, derive_more::Display)]
pub enum TobogganApiError {
    /// The server could not be reached: refused, unresolvable, or timed out.
    #[display("{_0}")]
    Transport(reqwest::Error),

    /// The server answered, and the answer was a refusal.
    ///
    /// Carries the body, because that is where the server explains itself and
    /// `error_for_status` throws it away.
    #[display("the server answered {code}{}", if body.is_empty() {
        String::new()
    } else {
        format!(": {body}")
    })]
    Status {
        code: StatusCode,
        #[error(not(source))]
        body: String,
    },

    /// The server answered, and the answer could not be read.
    ///
    /// Almost always a client and a server that disagree about a shape — which
    /// is a version skew, not a network fault.
    #[display("the server's answer could not be read: {_0}")]
    Decode(reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct TobogganApi {
    client: reqwest::Client,
    api_url: String,
    /// Offered as `Authorization: Bearer …` on every request.
    ///
    /// The same token the socket offers in `Register`, because `/api/command`
    /// and `/api/clients` — the two guarded endpoints this type can reach — are
    /// gated the same way the socket is.
    presenter_token: Option<Secret>,
}

/// How long to wait for a server to accept a connection.
///
/// Bounded because the alternative is unbounded: a host that drops packets
/// rather than refusing them answers nothing at all, and a caller waiting on
/// that has no way to tell it apart from a slow server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a whole request may take, connection included.
///
/// Generous, because `/api/slides` carries every slide's rendered HTML and a
/// large deck on a slow link is not an error. It exists so that a server which
/// accepts a connection and then goes quiet still ends the call.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

impl TobogganApi {
    #[must_use]
    pub fn new(api_url: impl Into<String>) -> Self {
        let api_url = api_url.into();
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            // Only fails if the TLS backend cannot start, which is exactly when
            // the default client would fail too. Nothing is lost by falling back.
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            api_url,
            presenter_token: None,
        }
    }

    /// Offers a presenter token on the REST side, as the socket already does.
    ///
    /// The server reads it from `Authorization: Bearer …`; see
    /// [`crate::TobogganConfig::with_presenter_token`], which is where a client
    /// usually gets one.
    #[must_use]
    pub fn with_presenter_token(mut self, token: Option<Secret>) -> Self {
        // Stored as given. Holding a `Secret` *is* the proof it is usable —
        // that is the type's entire job — so re-validating it would say
        // otherwise, and would mean exposing it here for no gain.
        self.presenter_token = token;
        self
    }

    /// The request with the token attached, when there is one to attach.
    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.presenter_token {
            Some(token) => request.bearer_auth(token.expose()),
            None => request,
        }
    }

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Turns an answer into the value it carries, or into the reason it cannot.
    ///
    /// The classification lives here so that every endpoint gets it: a refusal
    /// keeps its status *and* its body, and a shape the client cannot read is
    /// reported as such rather than as an unreachable server.
    async fn read<T>(response: reqwest::Response) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let status = response.status();
        if !status.is_success() {
            // Read before discarding: this is the only place the server's own
            // explanation exists, and `error_for_status` drops it.
            let body = response.text().await.unwrap_or_default();
            return Err(TobogganApiError::Status {
                code: status,
                body: body.trim().to_owned(),
            });
        }

        response.json().await.map_err(TobogganApiError::Decode)
    }

    async fn get<T>(&self, path: &str) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let response = self
            .authorized(self.client.get(&url))
            .send()
            .await
            .map_err(TobogganApiError::Transport)?;
        Self::read(response).await
    }

    async fn post<B, R>(&self, path: &str, body: &B) -> Result<R, TobogganApiError>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let url = self.build_url(path);
        let response = self
            .authorized(self.client.post(&url).json(body))
            .send()
            .await
            .map_err(TobogganApiError::Transport)?;
        Self::read(response).await
    }

    pub async fn talk(&self) -> Result<TalkResponse, TobogganApiError> {
        self.get("/api/talk").await
    }

    /// Who is currently connected.
    ///
    /// Unwrapped from the `{ "clients": [...] }` the endpoint returns; the test
    /// below pins that shape.
    ///
    /// Presenter-only on the server: an audience connection gets a 403 here,
    /// because the audience has no business enumerating the rest of the
    /// audience.
    pub async fn clients(&self) -> Result<Vec<ClientInfo>, TobogganApiError> {
        let response = self.get::<ClientsResponse>("/api/clients").await?;
        Ok(response.clients)
    }

    pub async fn slides(&self) -> Result<SlidesResponse, TobogganApiError> {
        self.get("/api/slides").await
    }

    pub async fn slide(&self, slide_id: SlideId) -> Result<Slide, TobogganApiError> {
        let path = format!("/api/slides/{}", slide_id.index());
        self.get(&path).await
    }

    pub async fn command(&self, command: Command) -> Result<Notification, TobogganApiError> {
        self.post("/api/command", &command).await
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::ClientRole;

    use super::*;

    /// A real `GET /api/clients` body, captured from the server.
    const CLIENTS_BODY: &str = r#"{
        "clients": [
            {
                "id": { "idx": 1, "version": 5 },
                "name": "Python",
                "ip_addr": "127.0.0.1",
                "connected_at": "2026-08-22T14:54:33.020245Z",
                "role": "Presenter"
            }
        ]
    }"#;

    /// The endpoint answers with an object wrapping the list, not a bare array.
    ///
    /// [`TobogganApi::clients`] asked for the array, so *every* call failed to
    /// deserialize. Nothing caught it because the only caller is the Python
    /// binding, where the failure surfaces as a `ConnectionError` in a REPL —
    /// a long way from this file. Both halves are asserted so that flattening
    /// the response one day fails here rather than there.
    #[test]
    fn the_clients_endpoint_wraps_its_list_in_an_object() {
        assert!(
            serde_json::from_str::<Vec<ClientInfo>>(CLIENTS_BODY).is_err(),
            "a bare array is what this used to ask for"
        );

        let response =
            serde_json::from_str::<ClientsResponse>(CLIENTS_BODY).expect("the wrapper object");
        let [client] = response.clients.as_slice() else {
            panic!("expected exactly one client");
        };
        assert_eq!(client.name, "Python");
        assert_eq!(client.role, ClientRole::Presenter);
    }
}
