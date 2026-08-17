//! Who is allowed to drive the deck.
//!
//! The rule is one sentence: **a connection from this machine presents, and a
//! connection from anywhere else presents only if it carries the token.**
//!
//! That keeps the everyday case free of ceremony — the server binds to
//! `127.0.0.1` by default, so every client is local and everything works as it
//! always has — while making the exposed case safe without asking the presenter
//! to remember anything. Binding to `0.0.0.0` to show the room the deck no
//! longer hands the room a shell on the presenter's laptop.
//!
//! # What this does not defend against
//!
//! A reverse proxy. Behind one, every connection arrives from the proxy — which
//! is usually loopback — so every client would present. Put a token on it, or
//! do not put a presentation server behind a proxy.

use std::net::IpAddr;
use std::sync::Arc;

use toboggan_core::ClientRole;

/// The server's answer to "may this connection drive the deck?".
///
/// Cheap to clone: it is pulled out of the shared state on every request.
#[derive(Debug, Clone, Default)]
pub struct PresenterAuth {
    token: Option<Arc<str>>,
}

impl PresenterAuth {
    /// Builds the gate from `--presenter-token`.
    ///
    /// An empty token is treated as none: an unset environment variable and one
    /// set to the empty string should not mean two different security postures,
    /// and `""` as a shared secret is not one.
    #[must_use]
    pub fn new(token: Option<&str>) -> Self {
        let token = token
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(Arc::from);
        Self { token }
    }

    /// Whether a remote client can become a presenter at all.
    #[must_use]
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// The role for a connection from `peer` offering `offered`.
    #[must_use]
    pub fn role_for(&self, peer: IpAddr, offered: Option<&str>) -> ClientRole {
        if is_local(peer) {
            return ClientRole::Presenter;
        }
        match (self.token.as_deref(), offered) {
            (Some(expected), Some(offered)) if tokens_match(expected, offered) => {
                ClientRole::Presenter
            }
            _ => ClientRole::Audience,
        }
    }
}

/// Whether `peer` is this machine.
///
/// The IPv6 arm is not paranoia: a dual-stack listener reports an IPv4 client
/// as `::ffff:127.0.0.1`, and [`std::net::Ipv6Addr::is_loopback`] is `false`
/// for that — so without the mapped case, binding to `::` would demote the
/// presenter's own browser to the audience.
fn is_local(peer: IpAddr) -> bool {
    match peer {
        IpAddr::V4(addr) => addr.is_loopback(),
        IpAddr::V6(addr) => {
            addr.is_loopback() || addr.to_ipv4_mapped().is_some_and(|addr| addr.is_loopback())
        }
    }
}

/// Compares two tokens without stopping at the first difference.
///
/// A short-circuiting `==` answers a wrong guess faster the earlier it is
/// wrong, which over enough attempts tells a guesser how much of the prefix
/// they have right. Folding every byte costs nothing here and removes the
/// question.
fn tokens_match(expected: &str, offered: &str) -> bool {
    let expected = expected.as_bytes();
    let offered = offered.as_bytes();
    // `zip` stops at the shorter of the two, so the lengths are folded in
    // separately rather than compared up front.
    let mut difference = expected.len() ^ offered.len();
    for (expected, offered) in expected.iter().zip(offered) {
        difference |= usize::from(expected ^ offered);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    const REMOTE: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42));

    /// The default posture, and the one that must not change: the server binds
    /// to loopback, so every client is this machine and everything presents.
    #[test]
    fn this_machine_always_presents() {
        let auth = PresenterAuth::new(None);
        assert_eq!(
            auth.role_for(IpAddr::V4(Ipv4Addr::LOCALHOST), None),
            ClientRole::Presenter
        );
        assert_eq!(
            auth.role_for(IpAddr::V6(Ipv6Addr::LOCALHOST), None),
            ClientRole::Presenter
        );
    }

    /// A dual-stack listener spells an IPv4 loopback client this way.
    #[test]
    fn a_mapped_ipv4_loopback_is_still_this_machine() {
        let mapped = IpAddr::V6(Ipv4Addr::LOCALHOST.to_ipv6_mapped());
        assert_eq!(
            PresenterAuth::new(None).role_for(mapped, None),
            ClientRole::Presenter
        );
    }

    /// The room watching over the wifi cannot move the deck, and cannot open a
    /// shell on the machine hosting it.
    #[test]
    fn the_network_watches_and_nothing_more() {
        let auth = PresenterAuth::new(None);
        assert_eq!(auth.role_for(REMOTE, None), ClientRole::Audience);
        // No token configured means no token can unlock it, however good the guess.
        assert_eq!(
            auth.role_for(REMOTE, Some("anything")),
            ClientRole::Audience
        );
    }

    #[test]
    fn a_remote_client_with_the_token_presents() {
        let auth = PresenterAuth::new(Some("s3cr3t"));
        assert_eq!(auth.role_for(REMOTE, Some("s3cr3t")), ClientRole::Presenter);
        assert_eq!(auth.role_for(REMOTE, Some("s3cr3")), ClientRole::Audience);
        assert_eq!(auth.role_for(REMOTE, Some("s3cr3t ")), ClientRole::Audience);
        assert_eq!(auth.role_for(REMOTE, None), ClientRole::Audience);
    }

    /// `TOBOGGAN_PRESENTER_TOKEN=` set but empty is "no token", not "the empty
    /// string is the password".
    #[test]
    fn an_empty_token_is_no_token() {
        let auth = PresenterAuth::new(Some("   "));
        assert!(!auth.has_token());
        assert_eq!(auth.role_for(REMOTE, Some("")), ClientRole::Audience);
        assert_eq!(auth.role_for(REMOTE, Some("   ")), ClientRole::Audience);
    }
}
