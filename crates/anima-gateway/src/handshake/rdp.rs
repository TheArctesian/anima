//! The `connect` values only RDP has: the domain, the security the device must use, the
//! rule for its certificate and the resize method (K4 Section 3).

use super::Refused;

/// The names of this file's values, in the order `args` lists them after the names that
/// VNC has too. They are names the hub sets for RDP (`tunnel/arguments.rs`). The hub's
/// names for sound and the shared drive are not offered, so the hub sends no value for
/// them: the gateway has neither.
pub(super) const ARGUMENTS: [&str; 5] = [
    "domain",
    "security",
    "ignore-cert",
    "cert-fingerprints",
    "resize-method",
];

/// The hash the hub names in front of the accepted fingerprint.
const FINGERPRINT_HASH: &str = "sha256";
/// The longest domain the gateway takes, in bytes.
const LONGEST_DOMAIN: usize = 255;

/// How the device must protect the session. Standard RDP security is never used: it has
/// no certificate to check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Security {
    /// TLS; the device asks for the sign-in itself.
    Tls,
    /// TLS, then the sign-in inside it before the session (NLA, with or without its
    /// extension).
    Nla,
    /// Whichever of the two the device picks.
    TlsOrNla,
}

impl Security {
    /// Whether the device may ask for the sign-in before the session, which needs the
    /// user name and the password at hand.
    #[must_use]
    pub const fn may_be_nla(self) -> bool {
        matches!(self, Self::Nla | Self::TlsOrNla)
    }
}

/// What the gateway does with the certificate the device shows (SEC-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Certificate {
    /// Only the certificate with this SHA-256 is taken: the one the operator accepted.
    Accepted([u8; 32]),
    /// Any certificate is taken: the operator set "ignore certificate" for the device.
    Any,
}

/// What `connect` says for an RDP session only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rdp {
    pub domain: Option<String>,
    pub security: Security,
    pub certificate: Certificate,
    /// A new size of the client's view is asked of the device (`display-update`). When
    /// off, the picture is scaled to the view.
    pub resize: bool,
}

impl Rdp {
    /// Reads one value for each name in [`ARGUMENTS`], in its order.
    pub(super) fn from_values(values: &[String]) -> Result<Self, Refused> {
        let bad = |argument| Refused::BadValue { argument };
        let [domain, security, ignore, fingerprints, resize] = values else {
            return Err(bad("connect"));
        };
        if domain.len() > LONGEST_DOMAIN {
            return Err(bad("domain"));
        }
        let security = match security.as_str() {
            "tls" => Security::Tls,
            // The default of guacd and of the hub's settings is NLA.
            "" | "nla" | "nla-ext" => Security::Nla,
            "any" => Security::TlsOrNla,
            "rdp" | "vmconnect" => return Err(Refused::SecurityNotServed),
            _ => return Err(bad("security")),
        };
        // An accepted certificate is the stricter rule: it wins when both are set.
        let certificate = match (fingerprints.as_str(), ignore.as_str()) {
            ("", "true") => Certificate::Any,
            ("", _) => return Err(bad("cert-fingerprints")),
            (text, _) => Certificate::Accepted(fingerprint(text).ok_or(bad("cert-fingerprints"))?),
        };
        Ok(Self {
            domain: (!domain.is_empty()).then(|| domain.clone()),
            security,
            certificate,
            resize: resize == "display-update",
        })
    }
}

/// The 32 bytes of `sha256:AA:BB:…`, the form the hub sends one accepted fingerprint in.
fn fingerprint(text: &str) -> Option<[u8; 32]> {
    let (hash, pairs) = text.split_once(':')?;
    if !hash.eq_ignore_ascii_case(FINGERPRINT_HASH) {
        return None;
    }
    let mut bytes = [0u8; 32];
    let mut pairs = pairs.split(':');
    for byte in &mut bytes {
        // `from_str_radix` alone would take a sign.
        let hex = |pair: &&str| pair.len() == 2 && pair.bytes().all(|b| b.is_ascii_hexdigit());
        let pair = pairs.next().filter(hex)?;
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    pairs.next().is_none().then_some(bytes)
}
