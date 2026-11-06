//! The pin decision (D-4) as a pure function, and the leaf fingerprint (D-5).

use std::fmt;

use sha2::{Digest, Sha256};

/// SHA-256 of a certificate's DER bytes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint([u8; 32]);

impl Fingerprint {
    pub fn of(der: &[u8]) -> Self {
        Self(Sha256::digest(der).into())
    }
}

/// 64 lower-case hex, the hub's form (H-4).
impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// An accepted certificate for one origin.
#[derive(Clone, Debug)]
pub struct Pin {
    pub host: String,
    pub port: u16,
    pub sha256: Fingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Not the pinned origin, or no pin and the system trusts it: the system decides.
    Default,
    /// The presented leaf is the pinned one.
    Accept,
    /// No pin and the system does not trust it: show the fingerprint, stop.
    Untrusted(Fingerprint),
    /// A pin exists and a different leaf was presented: stop and warn.
    Mismatch {
        expected: Fingerprint,
        presented: Fingerprint,
    },
}

/// What a server-trust challenge presented: the origin, the leaf and the system's opinion.
#[derive(Clone, Debug)]
pub struct Presented {
    pub host: String,
    pub port: u16,
    pub sha256: Fingerprint,
    pub system_trusted: bool,
}

/// A pin wins over system trust; any other host or port gets the system's handling.
pub fn decide(pin: Option<&Pin>, challenge: &Presented) -> Verdict {
    let presented = challenge.sha256;
    match pin {
        Some(pin)
            if pin.host.eq_ignore_ascii_case(&challenge.host) && pin.port == challenge.port =>
        {
            if pin.sha256 == presented {
                Verdict::Accept
            } else {
                Verdict::Mismatch {
                    expected: pin.sha256,
                    presented,
                }
            }
        }
        Some(_) => Verdict::Default,
        None if challenge.system_trusted => Verdict::Default,
        None => Verdict::Untrusted(presented),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(fp: Fingerprint) -> Pin {
        Pin {
            host: "127.0.0.1".into(),
            port: 8443,
            sha256: fp,
        }
    }

    fn at(host: &str, port: u16, sha256: Fingerprint, system_trusted: bool) -> Presented {
        Presented {
            host: host.into(),
            port,
            sha256,
            system_trusted,
        }
    }

    #[test]
    fn every_verdict() {
        let a = Fingerprint::of(b"a");
        let b = Fingerprint::of(b"b");
        assert_eq!(
            decide(None, &at("127.0.0.1", 8443, a, false)),
            Verdict::Untrusted(a)
        );
        assert_eq!(
            decide(None, &at("127.0.0.1", 8443, a, true)),
            Verdict::Default
        );
        assert_eq!(
            decide(Some(&pin(a)), &at("127.0.0.1", 8443, a, false)),
            Verdict::Accept
        );
        assert_eq!(
            decide(Some(&pin(a)), &at("127.0.0.1", 8443, b, true)),
            Verdict::Mismatch {
                expected: a,
                presented: b
            }
        );
        assert_eq!(
            decide(Some(&pin(a)), &at("127.0.0.1", 9443, b, false)),
            Verdict::Default
        );
        assert_eq!(
            decide(Some(&pin(a)), &at("localhost", 8443, b, false)),
            Verdict::Default
        );
    }

    #[test]
    fn hex_form() {
        let text = Fingerprint::of(b"").to_string();
        assert_eq!(
            text,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
