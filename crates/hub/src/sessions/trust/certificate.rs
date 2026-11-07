//! What the hub keeps of an RDP server's leaf certificate (SEC-007): the SHA-256 of its DER,
//! which is pinned, and its subject and expiry, which are shown and never trusted. Pure.

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

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

/// Format characters (Unicode Cf: the bidirectional controls, zero-width characters, the
/// byte order mark, tags and the like) and the line and paragraph separators (Zl, Zp), which
/// hide or reorder text or break a line when shown (as K4 Section 8.7 cleans titles).
fn is_invisible(c: char) -> bool {
    matches!(c,
        '\u{ad}' | '\u{600}'..='\u{605}' | '\u{61c}' | '\u{6dd}' | '\u{70f}'
        | '\u{890}'..='\u{891}' | '\u{8e2}' | '\u{180e}'
        | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{206f}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}'
        | '\u{110bd}' | '\u{110cd}' | '\u{13430}'..='\u{1343f}' | '\u{1bca0}'..='\u{1bca3}'
        | '\u{1d173}'..='\u{1d17a}' | '\u{e0001}' | '\u{e0020}'..='\u{e007f}')
}

/// The subject without control (Cc), format (Cf) or separator (Zl, Zp) characters, trimmed,
/// at most [`MAX_SUBJECT`] characters; `None` when nothing is left. The device chooses it, so
/// it must not break a line, hide text or reorder it wherever it is shown.
pub(in crate::sessions) fn cleaned_subject(text: &str) -> Option<String> {
    let kept: String = text
        .chars()
        .filter(|c| !c.is_control() && !is_invisible(*c))
        .collect();
    let kept: String = kept.trim().chars().take(MAX_SUBJECT).collect();
    let kept = kept.trim_end();
    (!kept.is_empty()).then(|| kept.to_owned())
}
