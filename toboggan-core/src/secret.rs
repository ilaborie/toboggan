use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A shared secret — today, the presenter token.
///
/// The type exists because the token did not have one. It travelled as an
/// `Option<String>` through eight `Debug`-deriving structs, and two of them
/// were logged whole: the server printed its entire settings at `INFO` on every
/// startup, and the WebSocket handler logged the raw text of every frame,
/// including the `Register` that carries the token. Redacting those two call
/// sites would have left the next one to be written.
///
/// So redaction lives in the type. [`Debug`] is written by hand rather than
/// derived, which is what makes it unbypassable: no `?token` or `?settings` in
/// any present or future format string can print the secret, and
/// [`expose`](Self::expose) is the one way to read it, so every deliberate use
/// is greppable.
///
/// Serialization is *not* redacted — `#[serde(transparent)]` keeps the wire
/// bytes exactly as they were, because offering the token is the whole point of
/// [`Command::Register`](crate::Command::Register). Redaction and serialization
/// answer different questions.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    /// The one definition of a usable token, applied wherever one arrives.
    ///
    /// Surrounding whitespace is dropped, and a token that is empty afterwards
    /// is no token at all: an unset environment variable and one set to the
    /// empty string should not mean two different security postures, and `""`
    /// as a shared secret is not one.
    ///
    /// This used to be implemented separately on each side, and the two sides
    /// disagreed. The server trimmed the token it was configured with and did
    /// *not* trim the one a client offered, so a token pasted with a trailing
    /// space — or arriving as `?token=abc%20` — could never match the token it
    /// was equal to, and the presenter was silently demoted to audience.
    ///
    /// ```
    /// # use toboggan_core::Secret;
    /// assert_eq!(Secret::new("  s3cr3t ").map(|s| s.expose().to_owned()),
    ///            Some("s3cr3t".to_owned()));
    /// assert!(Secret::new("   ").is_none());
    /// ```
    #[must_use]
    pub fn new(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_owned()))
    }

    /// Builds a secret from a `?token=…` value, as it arrives in a URL.
    ///
    /// Three clients read the token out of a query string and each decoded it
    /// differently: the web client used `decodeURIComponent`, which leaves `+`
    /// alone; the mobile client did not decode at all; and the server's own
    /// query extractor reads `+` as a space. A token containing a space or a
    /// `+` therefore could not survive the round trip, and the client was
    /// silently demoted to audience. This is the one decoder.
    ///
    /// ```
    /// # use toboggan_core::Secret;
    /// // `+` is a space in a query string, and `%20` is the same space.
    /// assert_eq!(Secret::from_query_value("a+b").map(|s| s.expose().to_owned()),
    ///            Some("a b".to_owned()));
    /// assert_eq!(Secret::from_query_value("a%2Bb").map(|s| s.expose().to_owned()),
    ///            Some("a+b".to_owned()));
    /// ```
    #[must_use]
    pub fn from_query_value(raw: &str) -> Option<Self> {
        Self::new(&percent_decode(raw))
    }

    /// The secret itself. Every call is a deliberate disclosure.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether `offered` is this secret, without stopping at the first
    /// difference.
    ///
    /// A short-circuiting `==` answers a wrong guess faster the earlier it is
    /// wrong, which over enough attempts tells a guesser how much of the prefix
    /// they have right. Folding every byte costs nothing here and removes the
    /// question. `offered` is normalised first, so the two sides of the
    /// comparison are built the same way.
    ///
    /// This is not a constant-time comparison in the cryptographic sense — the
    /// loop below is not shielded from the optimiser, and the byte count still
    /// depends on the input. It removes the prefix oracle, which is the attack
    /// a shared secret in a query string is actually exposed to.
    #[must_use]
    pub fn matches(&self, offered: &str) -> bool {
        let Some(offered) = Self::new(offered) else {
            return false;
        };
        let expected = self.0.as_bytes();
        let offered = offered.0.as_bytes();
        // `zip` stops at the shorter of the two, so the lengths are folded in
        // separately rather than compared up front.
        let mut difference = expected.len() ^ offered.len();
        for (expected, offered) in expected.iter().zip(offered) {
            difference |= usize::from(expected ^ offered);
        }
        difference == 0
    }
}

/// Decodes `application/x-www-form-urlencoded` text: `%XX` escapes, and `+` as
/// a space.
///
/// Bytes that do not form valid UTF-8 are left as they were written rather than
/// replaced, so a mistyped escape produces a token that fails to match instead
/// of one that matches something else.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        match byte {
            b'+' => out.push(b' '),
            // Only a well-formed `%XX` is an escape; anything else is the
            // per-cent sign the author actually typed.
            b'%' => match decode_escape(tail) {
                Some((decoded, remainder)) => {
                    out.push(decoded);
                    rest = remainder;
                }
                None => out.push(b'%'),
            },
            byte => out.push(byte),
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_owned())
}

/// Reads the two hex digits after a `%`, returning the byte and what follows.
fn decode_escape(after_percent: &[u8]) -> Option<(u8, &[u8])> {
    let (hex, rest) = after_percent.split_at_checked(2)?;
    let byte = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
    Some((byte, rest))
}

/// Never prints the secret. See the type's documentation.
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// Lets `clap` accept a [`Secret`] behind `--presenter-token` and its
/// environment variable, through the derive's automatic value parser.
impl FromStr for Secret {
    type Err = std::convert::Infallible;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Ok(Self(raw.trim().to_owned()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn surrounding_whitespace_is_not_part_of_the_secret() {
        let secret = Secret::new(" s3cr3t\n").expect("a token");
        assert_eq!(secret.expose(), "s3cr3t");
    }

    #[test]
    fn an_empty_token_is_no_token() {
        assert!(Secret::new("").is_none());
        assert!(Secret::new("  \t ").is_none());
    }

    /// The asymmetry that silently demoted a presenter: the configured token
    /// was trimmed and the offered one was not, so these two never matched.
    #[test]
    fn a_token_pasted_with_whitespace_still_matches() {
        let secret = Secret::new("s3cr3t").expect("a token");
        assert!(secret.matches("s3cr3t "));
        assert!(secret.matches(" s3cr3t"));
    }

    #[test]
    fn a_different_token_does_not_match() {
        let secret = Secret::new("s3cr3t").expect("a token");
        assert!(!secret.matches("wrong"));
        // Longer and shorter, because the length is folded in rather than
        // compared up front.
        assert!(!secret.matches("s3cr3"));
        assert!(!secret.matches("s3cr3tt"));
        assert!(!secret.matches(""));
    }

    /// The reason the type exists: `?settings` and `?command` print through
    /// `Debug`, and both were logged at `INFO`.
    #[test]
    fn debug_does_not_print_the_secret() {
        let secret = Secret::new("s3cr3t").expect("a token");
        assert!(!format!("{secret:?}").contains("s3cr3t"));
        // Including when it is nested in the struct that actually gets logged.
        assert!(!format!("{:?}", Some(secret)).contains("s3cr3t"));
    }

    /// The three clients decoded `?token=` three different ways, so a token
    /// with a space or a `+` in it could not survive the round trip.
    #[test]
    fn a_query_value_is_decoded_one_way() {
        let expected = Secret::new("a b").expect("a token");
        assert_eq!(Secret::from_query_value("a+b"), Some(expected.clone()));
        assert_eq!(Secret::from_query_value("a%20b"), Some(expected));

        let plus = Secret::new("a+b").expect("a token");
        assert_eq!(Secret::from_query_value("a%2Bb"), Some(plus));
    }

    /// A broken escape must not silently become a different token; it becomes
    /// one that does not match, and the client is told it is watching.
    #[test]
    fn a_malformed_escape_is_left_alone() {
        let secret = Secret::from_query_value("100%discount").expect("a token");
        assert_eq!(secret.expose(), "100%discount");
        assert!(Secret::from_query_value("").is_none());
    }

    /// Offering the token is the point, so the wire form must not be redacted —
    /// and must stay exactly what it was before the newtype existed.
    #[test]
    fn the_wire_form_is_the_bare_string() {
        let secret = Secret::new("s3cr3t").expect("a token");
        let json = serde_json::to_string(&secret).expect("serialize");
        assert_eq!(json, r#""s3cr3t""#);

        let parsed = serde_json::from_str::<Secret>(r#""s3cr3t""#).expect("deserialize");
        assert_eq!(parsed, secret);
    }
}
