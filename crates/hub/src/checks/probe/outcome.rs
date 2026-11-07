//! What a probe reports, and the typed reasons a probe fails.

use std::{io, net::IpAddr, time::Duration};

use chrono::{DateTime, Utc};

/// Longest failure message kept (CHK-003).
pub const MESSAGE_MAX: usize = 512;

/// Format characters (Unicode Cf) and line or paragraph separators (Zl, Zp): bidi
/// overrides, zero-width characters, the byte order mark and the like.
fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{00AD}' | '\u{061C}' | '\u{180E}'
        | '\u{200B}'..='\u{200F}'
        | '\u{2028}'..='\u{202E}'
        | '\u{2060}'..='\u{206F}'
        | '\u{FEFF}'
        | '\u{FFF9}'..='\u{FFFB}'
        | '\u{E0001}'..='\u{E007F}')
}

/// A message as stored: control, format and separator characters removed, at most [`MESSAGE_MAX`] characters.
#[must_use]
pub fn clean_message(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() && !is_invisible(*c))
        .take(MESSAGE_MAX)
        .collect()
}

/// How a probe judged its own attempt (D-2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Judgement {
    /// The probe tested the target and `ok` is its answer.
    #[default]
    Answered,
    /// An ok result that is not settled yet: Docker `health: starting`. The evaluation
    /// records it as Degraded.
    Starting,
    /// The hub could not test the target (no LocalAPI proxy). The evaluation records Unknown.
    Unjudged,
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
    pub judgement: Judgement,
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
            judgement: Judgement::Answered,
        }
    }

    /// A success that has no response time worth keeping: a listing read over SSH measures
    /// the link, not the thing checked.
    #[must_use]
    pub fn up_unmeasured() -> Self {
        Self {
            response_time: None,
            ..Self::up(Duration::ZERO)
        }
    }

    /// An ok result that is still starting: the message says what is pending (D-2).
    #[must_use]
    pub fn starting(failure: &Failure) -> Self {
        Self {
            ok: true,
            message: Some(clean_message(&failure.to_string())),
            judgement: Judgement::Starting,
            ..Self::up_unmeasured()
        }
    }

    /// A probe the hub could not run; the evaluation records Unknown with the message (D-2).
    #[must_use]
    pub fn unjudged(failure: &Failure) -> Self {
        Self {
            judgement: Judgement::Unjudged,
            ..Self::failed(failure)
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
            judgement: Judgement::Answered,
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
    #[error("a redirect left HTTPS while secrets are attached")]
    InsecureRedirect,
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
    /// The sealed HTTP secrets do not open for the check (SEC-003).
    #[error("the check secrets cannot be opened")]
    Secrets,
    /// The sealed HTTP secrets were given for another destination than the stored
    /// configuration names, so they are not sent (SEC-005).
    #[error("the check secrets do not match its configuration")]
    SecretsUnbound,
    /// A PUSH check got no push within its interval (CHK-002).
    #[error("no push received")]
    NoPush,
    /// Docker and systemd checks run through the SSH link, which the generic probe has not.
    #[error("this check runs through the SSH link")]
    NeedsLink,
    /// The link did not open or the command did not run. The error's text holds no host,
    /// address, device text or secret (LOG-007).
    #[error("the SSH link failed: {0}")]
    Link(String),
    #[error("the command output was longer than the hub reads")]
    OutputCut,
    #[error("the command exited with an error")]
    CommandFailed,
    #[error("the command output cannot be read")]
    OutputUnreadable,
    #[error("the container was not found")]
    ContainerNotFound,
    /// The state class is one of the fixed words of the Docker parser, never device text.
    #[error("the container is {0}")]
    ContainerState(&'static str),
    #[error("the container health check is still starting")]
    ContainerStarting,
    #[error("the unit was not found")]
    UnitNotFound,
    /// The state class is one of the fixed words of the systemd parser, never device text.
    #[error("the unit is {0}")]
    UnitState(&'static str),
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Connect(error.kind())
    }
}
