//! What a probe reports, and the typed reasons a probe fails.

use std::{io, net::IpAddr, time::Duration};

use chrono::{DateTime, Utc};

/// Longest failure message kept (CHK-003).
pub const MESSAGE_MAX: usize = 512;

/// A message as stored: control characters removed, at most [`MESSAGE_MAX`] characters.
#[must_use]
pub fn clean_message(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MESSAGE_MAX)
        .collect()
}

/// The raw result of one probe. Invert, retries and status are applied later.
///
/// A failed network attempt is `ok: false` with a short message; the message never holds a
/// header value, a body or a secret (LOG-007).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeResult {
    pub ok: bool,
    pub response_time: Option<Duration>,
    pub message: Option<String>,
    /// The address the probe used, for ICMP, TCP, TLS and HTTP.
    pub resolved_ip: Option<IpAddr>,
    /// Leaf certificate expiry, for TLS and HTTPS (CHK-007).
    pub cert_not_after: Option<DateTime<Utc>>,
}

impl ProbeResult {
    #[must_use]
    pub fn up(response_time: Duration) -> Self {
        Self {
            ok: true,
            response_time: Some(response_time),
            message: None,
            resolved_ip: None,
            cert_not_after: None,
        }
    }

    /// A failed probe. The message is stripped of control characters and capped.
    #[must_use]
    pub fn failed(failure: &Failure) -> Self {
        let text = clean_message(&failure.to_string());
        Self {
            ok: false,
            response_time: None,
            message: Some(text),
            resolved_ip: None,
            cert_not_after: None,
        }
    }

    #[must_use]
    pub fn via(mut self, ip: IpAddr) -> Self {
        self.resolved_ip = Some(ip);
        self
    }

    #[must_use]
    pub fn with_certificate(mut self, not_after: DateTime<Utc>) -> Self {
        self.cert_not_after = Some(not_after);
        self
    }
}

/// Why a probe failed. The messages hold no address, URL, header, body or secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Failure {
    #[error("the address is not permitted")]
    NotPermitted,
    #[error("the check timed out")]
    TimedOut,
    #[error("the name did not resolve")]
    Unresolved,
    #[error("the host has no address of the requested version")]
    NoAddress,
    #[error("the connection failed: {0}")]
    Connect(io::ErrorKind),
    #[error("the TLS handshake failed")]
    Handshake,
    #[error("the certificate has expired")]
    CertificateExpired,
    #[error("the certificate is not valid yet")]
    CertificateNotYetValid,
    #[error("the certificate cannot be read")]
    CertificateUnreadable,
    #[error("no ICMP reply")]
    NoReply,
    #[error("the ICMP socket is unavailable: {0}")]
    IcmpSocket(io::ErrorKind),
    #[error("the request failed")]
    Request,
    #[error("too many redirects")]
    TooManyRedirects,
    #[error("unexpected status {0}")]
    Status(u16),
    #[error("the keyword was not found in the first 1 MiB of the body")]
    KeywordMissing,
    #[error("the body is not JSON")]
    BodyNotJson,
    #[error("the JSON query found no value")]
    JsonQueryMissing,
    #[error("the JSON value is not the expected one")]
    JsonMismatch,
    #[error("a secret of the configuration is missing")]
    SecretMissing,
    #[error("the configuration is not valid")]
    BadConfig,
    #[error("the DNS lookup failed")]
    DnsLookup,
    #[error("the DNS answer is not the expected one")]
    DnsMismatch,
    #[error("this check type is not probed by the hub")]
    NotProbed,
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Connect(error.kind())
    }
}
