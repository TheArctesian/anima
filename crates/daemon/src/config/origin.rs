//! The hub's origin: HTTPS on port 443, or the Unix socket of `anima-link` in the link mode;
//! nothing after the host (MET-020, K8 Section 1).

use std::path::{Component, Path, PathBuf};

use reqwest::Url;

/// The hub's public origin when the operator names none (K8 Section 1).
pub const DEFAULT_HUB: &str = "https://hub.anima.ing";

/// What starts a link origin: `unix:` and the absolute path of the socket of `anima-link`.
const SOCKET_PREFIX: &str = "unix:";

/// The longest socket path: the smaller of the macOS and Linux limits, less the end byte.
const LONGEST_SOCKET_PATH: usize = 103;

/// An `https` origin, scheme and host on port 443, or a link origin, `unix:<path>` (MET-020);
/// with no user, path, query or fragment after a host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin(Spelled);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Spelled {
    Https(Url),
    LinkSocket(PathBuf),
    /// `http://127.0.0.1:<port>`, the link origin before R-172. A stored file can still hold it
    /// (ROL-018); the daemon never sends to the port.
    OldLinkPort(Url),
}

/// How the daemon reaches the hub of an [`Origin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach<'a> {
    /// HTTPS to this URL.
    Https(&'a Url),
    /// The socket of `anima-link` at this path.
    LinkSocket(&'a Path),
    /// The socket of `anima-link` in the daemon's own folder: the origin is an old link origin.
    OwnLinkSocket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("the hub origin is not a URL")]
    NotUrl,
    #[error(
        "the hub origin must start with https://, or be unix:<path> of the socket of anima-link"
    )]
    Scheme,
    #[error("the hub origin must be only scheme and host, like https://hub.anima.ing")]
    NotOrigin,
    #[error("the hub origin must use port 443")]
    Port,
    #[error(
        "the daemon no longer sends to a port of anima-link: start `anima-link device` with -daemon-socket <path> and give --hub unix:<path>"
    )]
    LinkPort,
    #[error(
        "the link origin must be unix: and the absolute path of the socket, like unix:/home/me/.config/anima-daemon/link.sock"
    )]
    SocketPath,
}

impl Origin {
    /// The origin `text` spells: `https`, or `unix:` and the path of the socket of the link.
    /// Refuses `http` to every host, and anything beyond scheme and host.
    ///
    /// # Errors
    /// [`OriginError`] naming what is wrong.
    pub fn parse(text: &str) -> Result<Self, OriginError> {
        match Self::parse_kept(text)? {
            Self(Spelled::OldLinkPort(_)) => Err(OriginError::LinkPort),
            origin => Ok(origin),
        }
    }

    /// The origin a file of the daemon holds: as [`Origin::parse`], and also the link origin of
    /// before R-172, `http://127.0.0.1:<port>`, which the daemon reaches by its own socket.
    ///
    /// # Errors
    /// [`OriginError`] naming what is wrong.
    pub fn parse_kept(text: &str) -> Result<Self, OriginError> {
        if let Some(path) = text.strip_prefix(SOCKET_PREFIX) {
            return socket_path(path).map(|path| Self(Spelled::LinkSocket(path)));
        }
        let url = Url::parse(text).map_err(|_| OriginError::NotUrl)?;
        let old_link = match url.scheme() {
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
        match (old_link, url.port()) {
            (false, None) => Ok(Self(Spelled::Https(url))),
            (false, Some(_)) => Err(OriginError::Port),
            (true, Some(_)) => Ok(Self(Spelled::OldLinkPort(url))),
            (true, None) => Err(OriginError::LinkPort),
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

    /// How the daemon reaches this hub. No answer names a TCP port of this computer.
    #[must_use]
    pub fn reach(&self) -> Reach<'_> {
        match &self.0 {
            Spelled::Https(url) => Reach::Https(url),
            Spelled::LinkSocket(path) => Reach::LinkSocket(path),
            Spelled::OldLinkPort(_) => Reach::OwnLinkSocket,
        }
    }

    /// The origin as text, as it was given, without a trailing slash: for the files of the
    /// daemon, which keep an old link origin as it is (ROL-018).
    #[must_use]
    pub fn as_text(&self) -> String {
        match &self.0 {
            Spelled::Https(url) | Spelled::OldLinkPort(url) => url.origin().ascii_serialization(),
            Spelled::LinkSocket(path) => format!("{SOCKET_PREFIX}{}", path.display()),
        }
    }
}

/// The path of a socket: absolute, with no `.`, `..` or empty part, and short enough to bind.
/// The count of the parts refuses what the path type drops without a word (`/./`, `//`).
#[cfg(unix)]
fn socket_path(text: &str) -> Result<PathBuf, OriginError> {
    let path = Path::new(text);
    let plain = path.is_absolute()
        && path.file_name().is_some()
        && !text.ends_with('/')
        && text.len() <= LONGEST_SOCKET_PATH
        && !text.chars().any(char::is_control)
        && path
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
        && path.components().count() == text.split('/').count();
    if plain {
        Ok(path.to_path_buf())
    } else {
        Err(OriginError::SocketPath)
    }
}

#[cfg(windows)]
fn socket_path(text: &str) -> Result<PathBuf, OriginError> {
    use std::path::Prefix;
    let normalized = text.replace('\\', "/");
    let path = Path::new(text);
    let drive = matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)));
    let plain = drive
        && path.is_absolute()
        && path.file_name().is_some()
        && !normalized.ends_with('/')
        && text.len() <= LONGEST_SOCKET_PATH
        && !text.chars().any(char::is_control)
        && normalized
            .split('/')
            .skip(1)
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'));
    if plain {
        Ok(path.to_path_buf())
    } else {
        Err(OriginError::SocketPath)
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
    fn k8_1_the_socket_of_the_link_is_an_origin() {
        let path = if cfg!(windows) {
            "C:/Users/me/AppData/Local/Anima/daemon/link.sock"
        } else {
            "/home/me/.config/anima-daemon/link.sock"
        };
        let origin = format!("unix:{path}");
        let link = Origin::parse(&origin).expect("a link");
        assert_eq!(link.as_text(), origin);
        assert_eq!(link.reach(), Reach::LinkSocket(Path::new(path)));
    }

    #[test]
    fn k8_1_a_socket_origin_is_one_plain_absolute_path() {
        let long = format!("unix:/{}", "a".repeat(LONGEST_SOCKET_PATH));
        for text in [
            "unix:",
            "unix:link.sock",
            "unix:./link.sock",
            "unix:/",
            "unix:/run/anima/",
            "unix:/run/../etc/link.sock",
            "unix:/run/./link.sock",
            "unix:/run//link.sock",
            "unix:///run/link.sock",
            "unix:/run/link.sock\n",
            long.as_str(),
        ] {
            assert_eq!(
                Origin::parse(text),
                Err(OriginError::SocketPath),
                "{text:?}"
            );
        }
    }

    #[test]
    fn k8_1_a_port_of_the_link_is_no_origin_to_enrol_at() {
        for text in [
            "http://127.0.0.1:8443",
            "http://127.0.0.1:1024",
            "http://127.0.0.1",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError::LinkPort), "{text}");
        }
    }

    #[test]
    fn rol_018_a_stored_old_link_origin_reads_keeps_its_text_and_names_no_port_to_reach() {
        let old = Origin::parse_kept("http://127.0.0.1:8443").expect("a stored old link origin");
        assert_eq!(old.as_text(), "http://127.0.0.1:8443");
        assert_eq!(old.reach(), Reach::OwnLinkSocket);
        let https = Origin::parse_kept(DEFAULT_HUB).expect("an https origin");
        assert!(
            matches!(https.reach(), Reach::Https(url) if url.as_str() == "https://hub.anima.ing/")
        );
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
            assert_eq!(Origin::parse_kept(text), Err(OriginError::Scheme), "{text}");
        }
    }

    #[test]
    fn met_020_an_old_link_origin_with_more_than_scheme_host_and_port_is_refused() {
        for text in [
            "http://127.0.0.1:8443/daemon",
            "http://user@127.0.0.1:8443",
            "http://127.0.0.1:8443/?x=1",
            "http://127.0.0.1:8443/#top",
        ] {
            assert_eq!(
                Origin::parse_kept(text),
                Err(OriginError::NotOrigin),
                "{text}"
            );
        }
    }

    #[test]
    fn met_020_an_origin_with_more_than_scheme_host_and_port_is_refused() {
        for text in [
            "https://hub.anima.ing/daemon",
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
