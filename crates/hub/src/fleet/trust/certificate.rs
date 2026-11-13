//! What the hub keeps of an RDP server's leaf certificate (SEC-007): the SHA-256 of its DER,
//! which is pinned, and its subject and expiry, which are shown and never trusted. Pure.

use chrono::{DateTime, Utc};

use super::{model::ServerCertificate, observations::Observation};
use crate::fingerprint::CertificateFingerprint;

/// Longest subject kept, in characters (`server_certificates_subject_check`).
pub(in crate::fleet::trust) const MAX_SUBJECT: usize = 1024;

/// The leaf certificate a server presented, as the hub keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PresentedCertificate {
    pub(crate) fingerprint: CertificateFingerprint,
    pub(crate) subject: Option<String>,
    pub(crate) not_after: Option<DateTime<Utc>>,
}

impl PresentedCertificate {
    /// The fingerprint of any bytes; the subject and expiry only when they parse as X.509.
    pub(crate) fn from_der(der: &[u8]) -> Self {
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
            fingerprint_sha256: seen.presented.fingerprint.colon_pairs(),
            subject: seen.presented.subject,
            not_after: seen.presented.not_after,
            first_seen_at: seen.first_seen_at,
        }
    }
}

/// The subject without control (Cc), format (Cf) or separator (Zl, Zp) characters, trimmed,
/// at most [`MAX_SUBJECT`] characters; `None` when nothing is left. The device chooses it, so
/// it must not break a line, hide text or reorder it wherever it is shown.
pub(in crate::fleet::trust) fn cleaned_subject(text: &str) -> Option<String> {
    let kept = crate::text::clean(text, usize::MAX);
    let kept: String = kept.trim().chars().take(MAX_SUBJECT).collect();
    let kept = kept.trim_end();
    (!kept.is_empty()).then(|| kept.to_owned())
}
