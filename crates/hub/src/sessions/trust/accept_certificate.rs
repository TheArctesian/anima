//! `acceptServerCertificate` (SEC-007, LOG-005): the operator confirms the fingerprint of the
//! certificate a connection test saw at the device's host and RDP port, and the hub stores it
//! with an audit entry, in one transaction.

use serde_json::json;
use sqlx::PgPool;

use super::{certificate::CertificateFingerprint, observations::Endpoint};
use crate::{
    events::{self, AuditAction, AuditTarget, NewAudit},
    ids::DeviceId,
    sessions::{
        Conflict, Error, ServerCertificate,
        end::Actor,
        find_device,
        live::Live,
        store::{self, AcceptedCertificate},
    },
};

/// The certificate the operator accepts: the device and the fingerprint they compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::sessions) struct CertificateAcceptance {
    pub(in crate::sessions) device: DeviceId,
    pub(in crate::sessions) fingerprint: CertificateFingerprint,
}

/// Accepts the certificate a connection test saw at the device's host and RDP port within
/// the last 15 minutes before `by.at`, when its fingerprint is the one given. What is stored
/// comes from that observation, never from the request. Replacing a certificate writes the
/// audit entry (`accept` on the device) with the old and new fingerprints; accepting the one
/// already stored changes nothing and writes nothing.
///
/// # Errors
/// `NotFound` for an unknown device; `Conflict` when no certificate was seen or it has
/// another fingerprint; `Database`, `Devices` or `Events` when a dependency fails.
pub(in crate::sessions) async fn accept_server_certificate(
    db: &PgPool,
    live: &Live,
    by: &Actor,
    acceptance: CertificateAcceptance,
) -> Result<ServerCertificate, Error> {
    let device = find_device(db, acceptance.device).await?;
    let seen = live
        .observed_certificate(&Endpoint::new(&device.host, device.rdp.port), by.at)
        .ok_or(Error::Conflict(Conflict::CertificateNotSeen))?;
    if seen.presented.fingerprint != acceptance.fingerprint {
        return Err(Error::Conflict(Conflict::CertificateDiffers));
    }
    let mut tx = db.begin().await?;
    store::lock_server_certificate(&mut tx, device.id).await?;
    let old = store::server_certificates(&mut *tx, &[device.id])
        .await?
        .remove(&device.id);
    let fingerprint = seen.presented.fingerprint.as_str();
    if let Some(old) = old
        .as_ref()
        .filter(|old| old.fingerprint_sha256 == fingerprint)
    {
        tx.commit().await?;
        return Ok(old.clone());
    }
    let certificate = ServerCertificate {
        fingerprint_sha256: fingerprint.to_owned(),
        subject: seen.presented.subject,
        not_after: seen.presented.not_after,
        first_seen_at: seen.first_seen_at,
    };
    let accepted = AcceptedCertificate {
        device: device.id,
        certificate: &certificate,
        by: by.operator.id,
    };
    store::put_server_certificate(&mut tx, &accepted).await?;
    let change = json!({
        "deviceName": device.name,
        "serverCertificate": {
            "oldFingerprint": old.map(|old| old.fingerprint_sha256),
            "newFingerprint": certificate.fingerprint_sha256,
        },
    });
    let audit = NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::Accept,
        target: AuditTarget::Device(device.id),
        client: by.client,
        change,
    };
    events::audit(&mut tx, by.at, audit).await?;
    tx.commit().await?;
    Ok(certificate)
}
