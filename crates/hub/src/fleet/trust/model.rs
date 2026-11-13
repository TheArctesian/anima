//! Domain types of trust: accepted host keys (SEC-006) and accepted RDP server certificates
//! (SEC-007 for RDP), and who accepts one.

use chrono::{DateTime, Utc};

use super::public_key::PublicKey;
use crate::fleet::events::{ClientAddr, Operator};

/// The host key the operator accepted for a device (SEC-006). Its algorithm and fingerprint
/// are read from the key itself, so they always agree with the key the hub pins (SEC-007).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostKey {
    pub public_key: PublicKey,
    /// When the hub first saw this key.
    pub first_seen_at: DateTime<Utc>,
}

/// The RDP server certificate the operator accepted for a device (SEC-007). Every field is what
/// the device's leaf certificate said when the hub read it; only the fingerprint is pinned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerCertificate {
    /// SHA-256 of the leaf certificate's DER: 32 lower-case hex pairs joined by colons, as
    /// `FreeRDP` writes it (`server_certificates_fingerprint_check`).
    pub fingerprint_sha256: String,
    /// The certificate's subject, as the device chose it; never trusted.
    pub subject: Option<String>,
    pub not_after: Option<DateTime<Utc>>,
    /// When the hub first saw this certificate.
    pub first_seen_at: DateTime<Utc>,
}

/// Who accepts, from where and when, as the schema data of a signed-in request gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::trust) struct Actor {
    pub(in crate::fleet::trust) operator: Operator,
    pub(in crate::fleet::trust) client: ClientAddr,
    pub(in crate::fleet::trust) at: DateTime<Utc>,
}
