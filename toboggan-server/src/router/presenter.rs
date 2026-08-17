//! The extractor that gates the privileged routes.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRef, FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use serde::Deserialize;
use tracing::warn;

use crate::auth::PresenterAuth;

/// Proof that the request may drive the deck.
///
/// Extracting it *is* the check: a caller that cannot present is refused with
/// `403` before the handler body runs. Written as an extractor rather than a
/// call at the top of each handler so that gating is part of a route's
/// signature — a privileged route that forgets to ask for it reads as
/// unprivileged, which is a thing a reviewer can see.
pub(super) struct Presenter;

/// Where a token may travel.
///
/// Both, because the two callers cannot use the same one: a `fetch` can set an
/// `Authorization` header, and a browser opening a `WebSocket` cannot set any
/// header at all — so the terminal socket has nowhere to put it but the query
/// string.
#[derive(Debug, Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

impl<S> FromRequestParts<S> for Presenter
where
    PresenterAuth: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        const REFUSED: (StatusCode, &str) = (
            StatusCode::FORBIDDEN,
            "This client is watching, not presenting. \
             Connect from the machine running the server, or pass the presenter token.",
        );

        // No `ConnectInfo` means the server was not started with
        // `into_make_service_with_connect_info`, so there is no peer address to
        // judge. Refusing is the only safe reading of "we cannot tell".
        let Some(&ConnectInfo(peer)) = parts.extensions.get::<ConnectInfo<SocketAddr>>() else {
            warn!("Refusing a privileged request: the peer address is unavailable");
            return Err(REFUSED);
        };

        let auth = PresenterAuth::from_ref(state);
        let token = offered_token(parts);
        if auth.role_for(peer.ip(), token.as_deref()).is_presenter() {
            return Ok(Self);
        }

        warn!(%peer, path = %parts.uri.path(), "Refused a privileged request");
        Err(REFUSED)
    }
}

/// Reads the token from `Authorization: Bearer …`, else from `?token=`.
fn offered_token(parts: &Parts) -> Option<String> {
    let bearer = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_owned);

    bearer.or_else(|| {
        Query::<TokenQuery>::try_from_uri(&parts.uri)
            .ok()
            .and_then(|Query(query)| query.token)
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use axum::http::Request;

    use super::*;

    fn parts_of(uri: &str, authorization: Option<&str>) -> Parts {
        let mut request = Request::get(uri);
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        request.body(()).expect("build request").into_parts().0
    }

    #[test]
    fn a_token_can_arrive_in_either_place() {
        assert_eq!(
            offered_token(&parts_of("/api/terminal?cols=80&token=abc", None)).as_deref(),
            Some("abc")
        );
        assert_eq!(
            offered_token(&parts_of("/api/command", Some("Bearer abc"))).as_deref(),
            Some("abc")
        );
        assert_eq!(offered_token(&parts_of("/api/command", None)), None);
    }

    /// A header beats the query string, so a token cannot be smuggled past a
    /// deliberate one by appending to the URL.
    #[test]
    fn the_header_wins_over_the_query_string() {
        assert_eq!(
            offered_token(&parts_of("/api/command?token=query", Some("Bearer header"))).as_deref(),
            Some("header")
        );
    }

    /// A token is arbitrary text in a URL, so it arrives percent-encoded.
    #[test]
    fn a_token_from_the_query_string_is_decoded() {
        assert_eq!(
            offered_token(&parts_of("/api/terminal?token=a%20b%2Bc", None)).as_deref(),
            Some("a b+c")
        );
    }

    /// Anything but `Bearer` is not a token we understand, and guessing is how
    /// a `Basic` credential ends up compared against a presenter token.
    #[test]
    fn only_a_bearer_credential_is_read() {
        assert_eq!(
            offered_token(&parts_of("/api/command", Some("Basic abc"))),
            None
        );
    }
}
