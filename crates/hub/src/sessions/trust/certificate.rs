//! What the hub keeps of an RDP server's leaf certificate (SEC-007): the SHA-256 of its DER,
//! which is pinned, and its subject and expiry, which are shown and never trusted. Pure.

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::observations::Observation;
use crate::sessions::ServerCertificate;

/// Longest subject kept, in characters (`server_certificates_subject_check`).
pub(in crate::sessions) const MAX_SUBJECT: usize = 1024;

/// SHA-256 of a certificate's DER as 32 lower-case hex pairs joined by colons, as `FreeRDP`
/// writes it (`server_certificates_fingerprint_check`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::sessions) struct CertificateFingerprint(String);

impl CertificateFingerprint {
    pub(in crate::sessions) fn of(der: &[u8]) -> Self {
        let pairs: Vec<String> = Sha256::digest(der)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Self(pairs.join(":"))
    }

    pub(in crate::sessions) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CertificateFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The text is not 32 hex pairs joined by colons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("not a SHA-256 certificate fingerprint")]
pub(in crate::sessions) struct BadCertificateFingerprint;

impl FromStr for CertificateFingerprint {
    type Err = BadCertificateFingerprint;

    /// Surrounding space is ignored and upper-case hex is read as lower case.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim().to_ascii_lowercase();
        let shaped = text.len() == 32 * 3 - 1
            && text.split(':').count() == 32
            && text
                .split(':')
                .all(|pair| pair.len() == 2 && pair.bytes().all(|byte| byte.is_ascii_hexdigit()));
        if shaped {
            Ok(Self(text))
        } else {
            Err(BadCertificateFingerprint)
        }
    }
}

/// The leaf certificate a server presented, as the hub keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct PresentedCertificate {
    pub(in crate::sessions) fingerprint: CertificateFingerprint,
    pub(in crate::sessions) subject: Option<String>,
    pub(in crate::sessions) not_after: Option<DateTime<Utc>>,
}

impl PresentedCertificate {
    /// The fingerprint of any bytes; the subject and expiry only when they parse as X.509.
    pub(in crate::sessions) fn from_der(der: &[u8]) -> Self {
        let parsed = x509_parser::parse_x509_certificate(der).ok();
        let subject = parsed
            .as_ref()
            .and_then(|(_, cert)| cleaned_subject(&cert.subject().to_string()));
        let not_after = parsed.as_ref().and_then(|(_, cert)| {
            DateTime::from_timestamp(cert.validity().not_after.timestamp(), 0)
        });
        Self {
            fingerprint: CertificateFingerprint::of(der),
            subject,
            not_after,
        }
    }
}

impl From<Observation<PresentedCertificate>> for ServerCertificate {
    /// The certificate as a connection test saw it, first seen when the hub first saw it.
    fn from(seen: Observation<PresentedCertificate>) -> Self {
        Self {
            fingerprint_sha256: seen.presented.fingerprint.as_str().to_owned(),
            subject: seen.presented.subject,
            not_after: seen.presented.not_after,
            first_seen_at: seen.first_seen_at,
        }
    }
}

/// The subject without control (Cc), format (Cf) or separator (Zl, Zp) characters, trimmed,
/// at most [`MAX_SUBJECT`] characters; `None` when nothing is left. The device chooses it, so
/// it must not break a line, hide text or reorder it wherever it is shown.
pub(in crate::sessions) fn cleaned_subject(text: &str) -> Option<String> {
    let kept = crate::text::clean(text, usize::MAX);
    let kept: String = kept.trim().chars().take(MAX_SUBJECT).collect();
    let kept = kept.trim_end();
    (!kept.is_empty()).then(|| kept.to_owned())
}
