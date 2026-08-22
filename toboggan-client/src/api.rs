use serde::Serialize;
use serde::de::DeserializeOwned;
use toboggan_core::{
    ClientInfo, ClientsResponse, Command, Notification, Secret, Slide, SlideId, SlidesResponse,
    TalkResponse,
};

#[derive(Debug, derive_more::Error, derive_more::From, derive_more::Display)]
pub enum TobogganApiError {
    ReqwestError(reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct TobogganApi {
    client: reqwest::Client,
    api_url: String,
    /// Offered as `Authorization: Bearer …` on every request.
    ///
    /// Only the socket used to carry the token, so `/api/command` and
    /// `/api/clients` — the two guarded endpoints this type can reach — were
    /// refused for *every* remote presenter, however good their token.
    presenter_token: Option<Secret>,
}

impl TobogganApi {
    #[must_use]
    pub fn new(api_url: impl Into<String>) -> Self {
        let api_url = api_url.into();
        let client = reqwest::Client::new();
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
        self.presenter_token = token.and_then(|token| Secret::new(token.expose()));
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

    async fn get<T>(&self, path: &str) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
        let response = self.authorized(self.client.get(&url)).send().await?;
        let response = response.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
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
            .await?;
        let response = response.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn talk(&self) -> Result<TalkResponse, TobogganApiError> {
        self.get("/api/talk").await
    }

    /// Who is currently connected.
    ///
    /// Unwrapped from the `{ "clients": [...] }` the endpoint actually returns.
    /// This asked for a bare array, so every call failed to deserialize — and
    /// the only caller is the Python binding, which is why nothing noticed.
    ///
    /// Presenter-only on the server: an audience connection gets a 403 here,
    /// because the room has no business enumerating the room.
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
