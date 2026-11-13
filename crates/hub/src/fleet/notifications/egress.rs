//! The one way out for notification connections (P3-003, D-17, N-1).
//!
//! Every webhook, ntfy, Apprise and e-mail connection is built here. The host is resolved once
//! with [`targets::resolve_checked`](crate::fleet::targets::resolve_checked), which refuses the host
//! when the target rule or the deny list refuses any address it names; the connection then
//! goes only to those checked addresses and never resolves the name again. TLS is always
//! verified against the built-in roots for the host name; there is no switch to skip it.
//!
//! Failures are [`SendError`]s, whose text names no host, address, URL, header, credential or
//! reply (LOG-007).

mod http;
mod smtp;
#[cfg(test)]
pub(super) mod tests;

pub(crate) use http::HttpRequest;
pub(crate) use smtp::{SmtpSecurity, SmtpServer};

use std::{error::Error, io, time::Duration};

use crate::fleet::targets::{AddressPolicy, TargetError};

/// The longest one notification connection may take, from the lookup to the last byte read
/// (D-17). `sendTestNotification` waits at most this long for its answer (K1).
pub(crate) const TIME_LIMIT: Duration = Duration::from_secs(8);

/// The rules a notification connection obeys: the address policy every resolved address must
/// pass. A test may also trust its own root certificate ([`Egress::trusting`]).
#[derive(Clone, Debug)]
pub(crate) struct Egress {
    policy: AddressPolicy,
    /// DER root certificates trusted besides the built-in roots. Only tests add one.
    extra_roots: Vec<Vec<u8>>,
}

impl Egress {
    pub(crate) fn new(policy: AddressPolicy) -> Self {
        Self {
            policy,
            extra_roots: Vec::new(),
        }
    }

    /// Also trusts the DER root certificate `der`, for tests that reach a local TLS server.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn trusting(mut self, der: Vec<u8>) -> Self {
        self.extra_roots.push(der);
        self
    }
}

/// Why a notification was not sent. The text names nothing the operator configured or the
/// server sent beyond a status or reply code (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SendError {
    /// The host does not resolve, or names an address the hub may not reach. The dispatcher
    /// reports it as "The target address is not permitted." (N-5, K7). Unresolved and refused
    /// are folded into one message on purpose: telling them apart would let an operator probe
    /// which internal names exist (no DNS oracle).
    #[error("the target address is not permitted")]
    Target(#[from] TargetError),
    #[error("the address is not an https URL with a host")]
    Url,
    /// The hub could not prepare the connection itself (its TLS settings or HTTP client); no
    /// connection was made.
    #[error("the notification connection could not be prepared")]
    Setup,
    #[error("the connection failed or broke")]
    Connect,
    #[error("TLS failed: the certificate is not trusted for the host name, or the handshake broke")]
    Tls,
    #[error("no answer within the time limit")]
    TimedOut,
    /// Any answer outside 2xx, a redirect included: no redirect is followed.
    #[error("the server answered with HTTP status {0}")]
    HttpStatus(u16),
    #[error("the mail server does not offer STARTTLS")]
    NoStartTls,
    #[error("the mail server refused with reply code {0}")]
    SmtpReply(u16),
    #[error("the server's answer was not understood")]
    Exchange,
}

/// Whether `error`, or anything it wraps, is a TLS failure.
fn tls_broke(error: &(dyn Error + 'static)) -> bool {
    causes(error).any(<dyn Error>::is::<rustls::Error>)
}

/// `error` and every error it wraps, outermost first. An I/O error leads to the error it
/// carries, which its own `source` skips.
fn causes<'a>(error: &'a (dyn Error + 'static)) -> impl Iterator<Item = &'a (dyn Error + 'static)> {
    std::iter::successors(Some(error), |&cause| {
        match cause.downcast_ref::<io::Error>() {
            Some(io) => io.get_ref().map(|inner| inner as &(dyn Error + 'static)),
            None => cause.source(),
        }
    })
}
