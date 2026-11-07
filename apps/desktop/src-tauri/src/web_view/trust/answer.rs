//! The answer a device web view gives a server-trust challenge (D-4, WEB-008): accept, cancel
//! or leave it to the system, and which verdict the operator should see. Pure: the platform
//! trust hook reads the challenge, this decides, the hook only carries the answer back.

use super::{
    fingerprint::Fingerprint,
    pin::{Challenge, Host, Pin, PinnedOrigin, Verdict, verdict},
};

/// The port a challenge without one is for: server trust is TLS, and TLS defaults to 443.
const TLS_PORT: u16 = 443;

/// A server-trust challenge as the platform reports it, before any check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presented<'a> {
    /// The host as the platform spells it.
    pub host: &'a str,
    /// The port as the platform gives it; 0 means none was set.
    pub port: isize,
    /// SHA-256 of the leaf certificate; `None` if the server presented no chain.
    pub leaf: Option<Fingerprint>,
}

/// What the platform tells the web view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Use the presented chain: the pinned certificate.
    Accept,
    /// Stop the connection; nothing from it loads.
    Cancel,
    /// Let the system decide, as it would with no hook.
    Default,
}

/// An answer and the verdict to show the operator, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ruling {
    pub answer: Answer,
    /// `Untrusted` for the tab's own origin, or `Mismatch`.
    pub shown: Option<Verdict>,
}

impl Ruling {
    const fn quiet(answer: Answer) -> Self {
        Self {
            answer,
            shown: None,
        }
    }
}

/// Decides a challenge in a tab with `pin` that was opened on `origin`.
///
/// Fails closed: no leaf, no host or a port out of range cancels. `system_trust` is asked
/// only when the tab has no pin, since a pin decides alone (D-4) and evaluating trust can
/// block. An untrusted certificate is shown only for the tab's own origin, the one a pin
/// would cover; on any other host it is just cancelled.
pub fn rule(
    pin: Option<&Pin>,
    origin: Option<&PinnedOrigin>,
    presented: &Presented<'_>,
    system_trust: impl FnOnce() -> bool,
) -> Ruling {
    let host = presented.host;
    let (Some(leaf), Some(port), Some(_)) = (
        presented.leaf,
        port_of(presented.port),
        Host::of_challenge(host),
    ) else {
        return Ruling::quiet(Answer::Cancel);
    };
    let challenge = Challenge {
        host,
        port,
        presented: leaf,
        system_trusted: pin.is_none() && system_trust(),
    };
    match verdict(pin, &challenge) {
        Verdict::Accept => Ruling::quiet(Answer::Accept),
        Verdict::Default => Ruling::quiet(Answer::Default),
        Verdict::Untrusted(_) if !origin.is_some_and(|origin| origin.holds(host, port)) => {
            Ruling::quiet(Answer::Cancel)
        }
        shown @ (Verdict::Untrusted(_) | Verdict::Mismatch { .. }) => Ruling {
            answer: Answer::Cancel,
            shown: Some(shown),
        },
    }
}

/// The challenge's port: 0 is the TLS default, anything outside `u16` is no port.
fn port_of(port: isize) -> Option<u16> {
    match port {
        0 => Some(TLS_PORT),
        port => u16::try_from(port).ok(),
    }
}
