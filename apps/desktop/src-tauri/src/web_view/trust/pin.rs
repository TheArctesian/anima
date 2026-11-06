//! What a device web view does with a server certificate (D-4, WEB-008): the pinned origin
//! and the verdict on a trust challenge. Pure: no I/O, no clock.

use std::net::IpAddr;

use tauri::Url;

use super::fingerprint::Fingerprint;

const HTTPS_PORT: u16 = 443;

/// A host as compared for pinning: a lower-case name without its trailing dot, or an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::web_view) enum Host {
    Name(String),
    Address(IpAddr),
}

impl Host {
    /// Reads a host as a URL or a trust challenge gives it: IPv6 with or without brackets,
    /// a name in any case, with or without one trailing dot.
    fn read(text: &str) -> Self {
        let bare = text
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .unwrap_or(text);
        match bare.parse::<IpAddr>() {
            Ok(address) => Self::Address(address),
            Err(_) => Self::Name(bare.strip_suffix('.').unwrap_or(bare).to_ascii_lowercase()),
        }
    }
}

/// The `https` host and port a pin belongs to (H-4: only `https` takes a pin; a change of
/// host or port drops it, a path change keeps it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedOrigin {
    pub(in crate::web_view) host: Host,
    pub(in crate::web_view) port: u16,
}

/// Why a URL has no pinnable origin. Carries no part of the URL (LOG-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("the address is not a URL")]
    Unreadable,
    #[error("the address is not https")]
    NotHttps,
    #[error("the address holds a user name or password")]
    Userinfo,
    #[error("the address has no host")]
    NoHost,
}

impl OriginError {
    /// Stable code the client switches on.
    pub fn code(self) -> &'static str {
        match self {
            Self::Unreadable => "url",
            Self::NotHttps => "scheme",
            Self::Userinfo => "userinfo",
            Self::NoHost => "host",
        }
    }
}

impl PinnedOrigin {
    /// The origin of an `https` URL; the port defaults to 443.
    pub fn parse(url: &str) -> Result<Self, OriginError> {
        let url = Url::parse(url).map_err(|_| OriginError::Unreadable)?;
        if url.scheme() != "https" {
            return Err(OriginError::NotHttps);
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(OriginError::Userinfo);
        }
        let host = url.host_str().ok_or(OriginError::NoHost)?;
        Ok(Self {
            host: Host::read(host),
            port: url.port().unwrap_or(HTTPS_PORT),
        })
    }

    /// Whether a challenge for this host and port belongs to this origin.
    fn holds(&self, host: &str, port: u16) -> bool {
        self.port == port && self.host == Host::read(host)
    }
}

/// The certificate the operator accepted for one device's web interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub origin: PinnedOrigin,
    pub fingerprint: Fingerprint,
}

/// A server-trust challenge as the web view reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Challenge<'a> {
    pub host: &'a str,
    pub port: u16,
    /// SHA-256 of the leaf certificate the server presented.
    pub presented: Fingerprint,
    /// Whether the system trusts the presented chain for this host.
    pub system_trusted: bool,
}

/// What the web view does with a challenge (D-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Leave the challenge to the system's own handling.
    Default,
    /// The pinned certificate: load the page, whatever the system thinks of it.
    Accept,
    /// No pin and not trusted: stop and show the fingerprint for a first-visit accept.
    Untrusted(Fingerprint),
    /// The pinned origin presented another certificate: stop and warn (as SEC-007).
    Mismatch {
        expected: Fingerprint,
        presented: Fingerprint,
    },
}

/// Decides a challenge. A pin wins over system trust on its own origin; any other host or
/// port gets the system's handling.
pub fn verdict(pin: Option<&Pin>, challenge: &Challenge<'_>) -> Verdict {
    match pin {
        Some(pin) if pin.origin.holds(challenge.host, challenge.port) => {
            if pin.fingerprint == challenge.presented {
                Verdict::Accept
            } else {
                Verdict::Mismatch {
                    expected: pin.fingerprint,
                    presented: challenge.presented,
                }
            }
        }
        Some(_) => Verdict::Default,
        None if challenge.system_trusted => Verdict::Default,
        None => Verdict::Untrusted(challenge.presented),
    }
}
