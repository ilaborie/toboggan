//! Who is allowed to drive the deck.
//!
//! The rule is one sentence: **a connection over loopback presents, and a
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
//!
//! A forwarded port. `ssh -L 8080:localhost:8080` makes a remote user's traffic
//! arrive over loopback, so they present without a token. Anyone who can open
//! that tunnel can already run commands on the machine, so this is a statement
//! of scope rather than a hole — but it is why the rule above says *loopback*
//! and not *this machine*.
//!
//! A page in the presenter's own browser. Script on any origin can make
//! requests to `127.0.0.1`, and they arrive over loopback like everything else.
//! That is [`crate::router`]'s origin guard, not this module's.

use std::net::IpAddr;

use toboggan_core::{ClientRole, Secret};

/// The server's answer to "may this connection drive the deck?".
///
/// Cheap to clone: it is pulled out of the shared state on every request.
#[derive(Debug, Clone, Default)]
pub struct PresenterAuth {
    token: Option<Secret>,
}

impl PresenterAuth {
    /// Builds the gate from `--presenter-token`.
    ///
    /// What counts as a usable token is [`Secret::new`]'s decision, so the
    /// token the server is configured with and the token a client offers are
    /// normalised the same way. They were not, once: this side was trimmed and
    /// the offered side was not, so a token pasted with a trailing space could
    /// not match the token it was equal to.
    #[must_use]
    pub fn new(token: Option<Secret>) -> Self {
        // Re-normalised rather than trusted: `--presenter-token ""` and an
        // environment variable set to nothing both arrive as a `Secret` that
        // holds no secret, and they must mean the same as not passing it.
        Self {
            token: token.and_then(|token| Secret::new(token.expose())),
        }
    }

    /// Whether a remote client can become a presenter at all.
    #[must_use]
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// The token encoded for a `?token=` link, for printing to the operator.
    ///
    /// The one place the server discloses it, and it discloses it to the person
    /// who set it, on their own console. Without this they assemble the URL by
    /// hand — which is where the stray whitespace and the encoding mismatches
    /// came from.
    #[must_use]
    pub fn token_for_link(&self) -> Option<String> {
        self.token.as_ref().map(Secret::to_query_value)
    }

    /// The role for a connection from `peer` offering `offered`.
    #[must_use]
    pub fn role_for(&self, peer: IpAddr, offered: Option<&str>) -> ClientRole {
        if is_local(peer) {
            return ClientRole::Presenter;
        }
        match (self.token.as_ref(), offered) {
            (Some(expected), Some(offered)) if expected.matches(offered) => ClientRole::Presenter,
            _ => ClientRole::Audience,
        }
    }
}

/// Whether `peer` reached us over loopback.
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
        let auth = PresenterAuth::new(Secret::new("s3cr3t"));
        assert_eq!(auth.role_for(REMOTE, Some("s3cr3t")), ClientRole::Presenter);
        assert_eq!(auth.role_for(REMOTE, Some("s3cr3")), ClientRole::Audience);
        assert_eq!(auth.role_for(REMOTE, None), ClientRole::Audience);
    }

    /// This used to assert `Audience`, which was the bug: the configured token
    /// was trimmed on the way in and the offered one never was, so a token
    /// pasted with a trailing space — or arriving as `?token=s3cr3t%20` — could
    /// not match the token it was equal to. Both sides go through
    /// [`Secret::new`] now, and the presenter is no longer demoted for it.
    #[test]
    fn a_token_pasted_with_whitespace_still_presents() {
        let auth = PresenterAuth::new(Secret::new("s3cr3t"));
        assert_eq!(
            auth.role_for(REMOTE, Some("s3cr3t ")),
            ClientRole::Presenter
        );
        assert_eq!(
            auth.role_for(REMOTE, Some(" s3cr3t")),
            ClientRole::Presenter
        );
    }

    /// `TOBOGGAN_PRESENTER_TOKEN=` set but empty is "no token", not "the empty
    /// string is the password". Parsed the way clap parses it, so this covers
    /// the flag as it actually arrives.
    #[test]
    fn an_empty_token_is_no_token() {
        let auth = PresenterAuth::new("   ".parse::<Secret>().ok());
        assert!(!auth.has_token());
        assert_eq!(auth.role_for(REMOTE, Some("")), ClientRole::Audience);
        assert_eq!(auth.role_for(REMOTE, Some("   ")), ClientRole::Audience);
    }
}
