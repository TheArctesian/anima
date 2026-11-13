//! `acceptServerCertificate` (SEC-007, LOG-005): the operator confirms the fingerprint of the
//! certificate a connection test saw at the device's host and RDP port, and the hub stores it
//! with an audit entry, in one transaction.

use serde_json::json;
use sqlx::PgPool;

use super::{
    Conflict, Error, find_device,
    model::{Actor, ServerCertificate},
    observations::Endpoint,
    sightings::Sightings,
    store::{self, AcceptedCertificate},
};
use crate::{
    fingerprint::CertificateFingerprint,
    fleet::events::{self, AuditAction, AuditTarget, NewAudit},
    ids::DeviceId,
};

/// The certificate the operator accepts: the device and the fingerprint they compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::trust) struct CertificateAcceptance {
    pub(in crate::fleet::trust) device: DeviceId,
    pub(in crate::fleet::trust) fingerprint: CertificateFingerprint,
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
pub(in crate::fleet::trust) async fn accept_server_certificate(
    db: &PgPool,
    sightings: &Sightings,
    by: &Actor,
    acceptance: CertificateAcceptance,
) -> Result<ServerCertificate, Error> {
    let device = find_device(db, acceptance.device).await?;
    let seen = sightings
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
    let certificate = ServerCertificate::from(seen);
    if let Some(old) = old
        .as_ref()
        .filter(|old| old.fingerprint_sha256 == certificate.fingerprint_sha256)
    {
        tx.commit().await?;
        return Ok(old.clone());
    }
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
