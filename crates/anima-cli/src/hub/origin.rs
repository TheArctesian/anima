//! The hub's origin: `https` and a host, nothing after it (CMD-013). Plain `http` only for this
//! computer, so tests can run a hub on a loopback port.

use std::fmt;

use reqwest::Url;

/// The hub when the operator names none (CMD-013, DNS-001).
pub const DEFAULT_HUB: &str = "https://hub.anima.ing";

/// Hosts that may use plain `http`: they never leave this computer.
const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// A hub origin such as `https://hub.anima.ing` or `https://10.10.0.2:8443`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin(Url);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("the hub address is not a URL")]
    NotUrl,
    #[error("the hub address must start with https:// (http only for localhost)")]
    NotHttps,
    #[error("the hub address must be only a scheme and a host, like https://hub.anima.ing")]
    NotOrigin,
}

impl Origin {
    /// The origin `text` spells. One trailing `/` is allowed; a path, query, fragment or user
    /// name is not.
    ///
    /// # Errors
    /// [`OriginError`] naming what is wrong.
    pub fn parse(text: &str) -> Result<Self, OriginError> {
        let url = Url::parse(text.trim()).map_err(|_| OriginError::NotUrl)?;
        let host = url.host_str().ok_or(OriginError::NotOrigin)?;
        let secure =
            url.scheme() == "https" || (url.scheme() == "http" && LOOPBACK.contains(&host));
        if !secure {
            return Err(OriginError::NotHttps);
        }
        let bare = url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/";
        if !bare {
            return Err(OriginError::NotOrigin);
        }
        Ok(Self(url))
    }

    /// [`DEFAULT_HUB`].
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "DEFAULT_HUB is a constant that the tests parse"
    )]
    pub fn default_hub() -> Self {
        Self::parse(DEFAULT_HUB).expect("the default hub is an https origin")
    }

    /// The GraphQL endpoint, `POST /graphql` (K7).
    #[must_use]
    pub fn graphql(&self) -> Url {
        let mut url = self.0.clone();
        url.set_path("/graphql");
        url
    }

    /// The origin as text without a trailing slash: the key-store account and the config value.
    #[must_use]
    pub fn as_text(&self) -> String {
        self.0.origin().ascii_serialization()
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_013_the_default_hub_is_an_https_origin() {
        assert_eq!(Origin::default_hub().as_text(), DEFAULT_HUB);
    }

    #[test]
    fn cmd_013_https_origins_are_kept_without_the_trailing_slash() {
        let origin = Origin::parse("https://hub.example:8443/").expect("origin");
        assert_eq!(origin.as_text(), "https://hub.example:8443");
        assert_eq!(
            origin.graphql().as_str(),
            "https://hub.example:8443/graphql"
        );
    }

    #[test]
    fn cmd_013_plain_http_is_only_for_this_computer() {
        for text in [
            "http://localhost:8080",
            "http://127.0.0.1:1",
            "http://[::1]:9",
        ] {
            assert!(Origin::parse(text).is_ok(), "{text}");
        }
        for text in ["http://hub.example", "http://10.0.0.1", "ftp://hub.example"] {
            assert_eq!(Origin::parse(text), Err(OriginError::NotHttps), "{text}");
        }
    }

    #[test]
    fn cmd_014_an_origin_holds_no_path_query_or_user() {
        for text in [
            "https://hub.example/graphql",
            "https://hub.example/?a=1",
            "https://hub.example/#x",
            "https://user:secret@hub.example",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::NotOrigin), "{text}");
        }
        assert_eq!(Origin::parse("hub.example"), Err(OriginError::NotUrl));
    }
}
