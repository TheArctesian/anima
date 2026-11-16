//! The hub's origin: HTTPS on port 443, or the local port of `anima-link` in the link mode;
//! nothing after the host (MET-020, K8 Section 1).

use reqwest::Url;

/// The hub's public origin when the operator names none (K8 Section 1).
pub const DEFAULT_HUB: &str = "https://hub.anima.ing";

/// The lowest port a link origin may name. It only refuses a port that `anima-link`, which runs
/// as a plain user, does not listen on; it protects nothing.
const LOWEST_LINK_PORT: u16 = 1024;

/// An `https` origin, scheme and host on port 443, or a link origin, `http://127.0.0.1:<port>`
/// (MET-020); with no user, path, query or fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin(Url);

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("the hub origin is not a URL")]
    NotUrl,
    #[error("the hub origin must start with https://, or be http://127.0.0.1:<port> of anima-link")]
    Scheme,
    #[error("the hub origin must be only scheme and host, like https://hub.anima.ing")]
    NotOrigin,
    #[error("the hub origin must use port 443")]
    Port,
    #[error(
        "the link origin must name the port of anima-link, from 1024, like http://127.0.0.1:8443"
    )]
    LinkPort,
}

impl Origin {
    /// The origin `text` spells; refuses `http` to any host but `127.0.0.1`, and anything beyond
    /// scheme, host and port.
    ///
    /// # Errors
    /// [`OriginError`] naming what is wrong.
    pub fn parse(text: &str) -> Result<Self, OriginError> {
        let url = Url::parse(text).map_err(|_| OriginError::NotUrl)?;
        let link = match url.scheme() {
            "https" => false,
            // The URL parser spells every form of an IPv4 address as four decimal numbers.
            "http" if url.host_str() == Some("127.0.0.1") => true,
            _ => return Err(OriginError::Scheme),
        };
        let bare = url.host_str().is_some()
            && !text.contains('@')
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/";
        if !bare {
            return Err(OriginError::NotOrigin);
        }
        match (link, url.port()) {
            (false, None) => Ok(Self(url)),
            (false, Some(_)) => Err(OriginError::Port),
            (true, Some(port)) if port >= LOWEST_LINK_PORT => Ok(Self(url)),
            (true, _) => Err(OriginError::LinkPort),
        }
    }

    /// [`DEFAULT_HUB`].
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "DEFAULT_HUB is a constant that the tests parse"
    )]
    pub fn default_hub() -> Self {
        Self::parse(DEFAULT_HUB).expect("the default hub origin is an https origin")
    }

    /// The origin as a URL, the base the hub client joins its routes to.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.0
    }

    /// The origin as text without a trailing slash, for the config file.
    #[must_use]
    pub fn as_text(&self) -> String {
        self.0.origin().ascii_serialization()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_020_the_default_hub_is_an_https_origin() {
        assert_eq!(Origin::default_hub().as_text(), DEFAULT_HUB);
    }

    #[test]
    fn met_020_a_non_https_origin_is_refused_at_parse_time() {
        for text in [
            "http://hub.anima.ing",
            "ws://hub.anima.ing",
            "ftp://hub.anima.ing",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::Scheme), "{text}");
        }
        assert_eq!(Origin::parse("hub.anima.ing"), Err(OriginError::NotUrl));
    }

    #[test]
    fn met_020_the_local_port_of_the_link_is_an_origin() {
        let link = Origin::parse("http://127.0.0.1:8443").expect("a link origin");
        assert_eq!(link.as_text(), "http://127.0.0.1:8443");
        assert_eq!(link.url().as_str(), "http://127.0.0.1:8443/");
    }

    #[test]
    fn met_020_http_reaches_no_host_but_this_computer() {
        for text in [
            "http://localhost:8443",
            "http://127.0.0.2:8443",
            "http://[::1]:8443",
            "http://0.0.0.0:8443",
            "http://192.0.2.7:8443",
            "http://127.0.0.1.example:8443",
            "http://evil.example\\@127.0.0.1:8443",
            "http://127.0.0.1:8443@evil.example",
            "http://[::ffff:127.0.0.1]:8443",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::Scheme), "{text}");
        }
    }

    #[test]
    fn met_020_a_link_origin_names_a_port_from_1024() {
        for text in [
            "http://127.0.0.1",
            "http://127.0.0.1:80",
            "http://127.0.0.1:0",
            "http://127.0.0.1:1023",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::LinkPort), "{text}");
        }
        assert!(Origin::parse("http://127.0.0.1:1024").is_ok());
        assert!(Origin::parse("http://127.0.0.1:65535").is_ok());
    }

    #[test]
    fn met_020_a_link_origin_with_more_than_scheme_host_and_port_is_refused() {
        for text in [
            "http://127.0.0.1:8443/agent",
            "http://user@127.0.0.1:8443",
            "http://127.0.0.1:8443/?x=1",
            "http://127.0.0.1:8443/#top",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::NotOrigin), "{text}");
        }
    }

    #[test]
    fn met_020_an_origin_with_more_than_scheme_host_and_port_is_refused() {
        for text in [
            "https://hub.anima.ing/agent",
            "https://user:pw@hub.anima.ing",
            "https://hub.anima.ing/?x=1",
            "https://hub.anima.ing/#top",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::NotOrigin), "{text}");
        }
    }

    #[test]
    fn met_020_any_port_but_443_is_refused() {
        for text in [
            "https://hub.example:8443",
            "https://hub.anima.ing:22",
            "https://hub.anima.ing:0",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::Port), "{text}");
        }
        let explicit = Origin::parse("https://hub.anima.ing:443").expect("443 is the default");
        assert_eq!(explicit.as_text(), DEFAULT_HUB);
    }

    #[test]
    fn met_020_a_user_or_password_in_the_origin_is_refused() {
        for text in [
            "https://user@hub.anima.ing",
            "https://:pw@hub.anima.ing",
            "https://hub.anima.ing@evil.example",
            "https://:@hub.anima.ing",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::NotOrigin), "{text}");
        }
    }

    #[test]
    fn met_020_backslash_and_odd_spellings_do_not_move_the_host() {
        for text in [
            "https://evil.example\\@hub.anima.ing",
            "https://evil.example\\hub.anima.ing",
            "https://hub.anima.ing/.",
            "https://hub.anima.ing/%2e",
            "https:hub.anima.ing",
        ] {
            let parsed = Origin::parse(text);
            assert!(
                parsed
                    .as_ref()
                    .ok()
                    .is_none_or(|origin| origin.as_text() == DEFAULT_HUB),
                "{text} -> {parsed:?}"
            );
        }
        let upper =
            Origin::parse("HTTPS://HUB.ANIMA.ING").expect("scheme and host fold to lower case");
        assert_eq!(upper.as_text(), DEFAULT_HUB);
    }
}
