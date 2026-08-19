//! Refusing the requests another site made on the presenter's behalf.
//!
//! [`super::presenter::Presenter`] answers "may this peer drive the deck?" with
//! the peer's IP address, and a connection over loopback always may. That is
//! the right answer for a person at the keyboard and the wrong one for a page
//! they happened to open: script on any origin can reach `127.0.0.1`, and the
//! request arrives from loopback because the browser is on loopback.
//!
//! CORS does not close this. A preflight only decides whether the *response* is
//! readable, so a `POST /api/command` still moves the deck even when the reply
//! is thrown away — and a WebSocket handshake is not subject to CORS at all,
//! which is how `/api/terminal` becomes a shell on the presenter's laptop from
//! an ordinary web page.
//!
//! So the origin is checked directly, on the routes that can do damage. A
//! browser labels every cross-origin request with `Origin`; a request without
//! one did not come from a page, which is the ordinary case for the TUI, the
//! desktop client and `curl`.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use tracing::warn;

/// The origins a browser is allowed to drive the deck from, beyond the server's
/// own. Built from `--allowed-origins`.
#[derive(Debug, Clone, Default)]
pub(super) struct AllowedOrigins(Arc<[String]>);

impl AllowedOrigins {
    /// Builds the list from the `--allowed-origins` setting.
    pub(super) fn new(origins: Option<&[String]>) -> Self {
        Self(origins.unwrap_or_default().into())
    }

    /// Whether `origin` was named on the command line.
    fn permits(&self, origin: &str) -> bool {
        self.0.iter().any(|allowed| allowed == origin)
    }
}

const REFUSED: (StatusCode, &str) = (
    StatusCode::FORBIDDEN,
    "This request came from another site. Only the page the server itself \
     serves may drive the deck; pass --allowed-origins to widen that.",
);

/// Refuses a labelled request whose origin is neither ours nor allow-listed.
pub(super) async fn guard_origin(
    State(allowed): State<AllowedOrigins>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let Some(origin) = headers.get(header::ORIGIN) else {
        // Unlabelled: not a browser, so there is no other site to be acting for.
        return next.run(request).await;
    };
    let Ok(origin) = origin.to_str() else {
        warn!("Refused a request whose Origin is not text");
        return REFUSED.into_response();
    };

    let host = headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok());
    if host.is_some_and(|host| is_same_origin(origin, host)) || allowed.permits(origin) {
        return next.run(request).await;
    }

    warn!(
        origin,
        path = %request.uri().path(),
        "Refused a request from another origin",
    );
    REFUSED.into_response()
}

/// Whether `origin` names the same host and port the request was addressed to.
///
/// Compared as text rather than parsed: `Origin` is `scheme://host[:port]` and
/// `Host` is `host[:port]`, so dropping the scheme leaves two spellings of the
/// same authority. A scheme-less or malformed `Origin` — `null`, from a
/// sandboxed frame — has no authority to match and is refused.
fn is_same_origin(origin: &str, host: &str) -> bool {
    origin
        .split_once("://")
        .is_some_and(|(_, authority)| authority == host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_the_server_served_is_the_same_origin() {
        assert!(is_same_origin("http://127.0.0.1:8080", "127.0.0.1:8080"));
        assert!(is_same_origin("http://localhost:8080", "localhost:8080"));
        assert!(is_same_origin("https://talks.example", "talks.example"));
    }

    /// The whole point: a page somewhere else, talking to the deck on loopback.
    #[test]
    fn another_site_is_not() {
        assert!(!is_same_origin("https://evil.example", "127.0.0.1:8080"));
        // A different port is a different origin, however alike it reads.
        assert!(!is_same_origin("http://127.0.0.1:9999", "127.0.0.1:8080"));
    }

    /// A sandboxed frame sends `Origin: null`, which names no authority.
    #[test]
    fn an_origin_with_no_authority_matches_nothing() {
        assert!(!is_same_origin("null", "127.0.0.1:8080"));
        assert!(!is_same_origin("", "127.0.0.1:8080"));
    }

    #[test]
    fn an_allow_listed_origin_is_permitted() {
        let allowed = AllowedOrigins::new(Some(&["https://talks.example".to_owned()]));
        assert!(allowed.permits("https://talks.example"));
        assert!(!allowed.permits("https://evil.example"));
    }

    #[test]
    fn nothing_is_allow_listed_by_default() {
        assert!(!AllowedOrigins::new(None).permits("https://talks.example"));
    }
}
