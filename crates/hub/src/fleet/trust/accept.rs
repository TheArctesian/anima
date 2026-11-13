//! `acceptHostKey` (D-6, SEC-006, LOG-005): the operator confirms the fingerprint of the key a
//! connection test saw for the device's host and port, and the hub stores that key with an
//! audit entry, in one transaction.

use serde_json::json;
use sqlx::PgPool;

use super::{
    Conflict, Error, find_device,
    model::{Actor, HostKey},
    observations::Endpoint,
    public_key::Fingerprint,
    sightings::Sightings,
    store::{self, AcceptedKey},
};
use crate::{
    fleet::events::{self, AuditAction, AuditTarget, NewAudit},
    ids::DeviceId,
};

/// The key the operator accepts: the device and the fingerprint they compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::fleet::trust) struct Acceptance {
    pub(in crate::fleet::trust) device: DeviceId,
    pub(in crate::fleet::trust) fingerprint: Fingerprint,
}

/// Accepts the key a connection test saw at the device's host and port within the last
/// 15 minutes before `by.at`, when its fingerprint is the one given. The stored key, its
/// algorithm and fingerprint come from that observation, never from the request. Replacing
/// a key writes the audit entry `host_key.accept` with the old and new fingerprints; accepting
/// the key already stored changes nothing and writes nothing.
///
/// # Errors
/// `NotFound` for an unknown device; `Conflict` when no key was seen or it has another
/// fingerprint; `Database`, `Devices` or `Events` when a dependency fails.
pub(in crate::fleet::trust) async fn accept_host_key(
    db: &PgPool,
    sightings: &Sightings,
    by: &Actor,
    acceptance: Acceptance,
) -> Result<HostKey, Error> {
    let device = find_device(db, acceptance.device).await?;
    let seen = sightings
        .observed(&Endpoint::new(&device.host, device.ssh.port), by.at)
        .ok_or(Error::Conflict(Conflict::KeyNotSeen))?;
    if seen.presented.fingerprint() != acceptance.fingerprint {
        return Err(Error::Conflict(Conflict::KeyDiffers));
    }
    let mut tx = db.begin().await?;
    store::lock_host_key(&mut tx, device.id).await?;
    let old = store::host_keys(&mut *tx, &[device.id])
        .await?
        .remove(&device.id);
    if let Some(old) = old.as_ref().filter(|old| old.public_key == seen.presented) {
        tx.commit().await?;
        return Ok(old.clone());
    }
    let key = HostKey {
        public_key: seen.presented,
        first_seen_at: seen.first_seen_at,
    };
    let accepted = AcceptedKey {
        device: device.id,
        key: &key,
        by: by.operator.id,
        at: by.at,
    };
    store::put_host_key(&mut tx, &accepted).await?;
    let change = json!({
        "deviceName": device.name,
        "algorithm": key.public_key.algorithm(),
        "oldFingerprint": old.map(|old| old.public_key.fingerprint().to_string()),
        "newFingerprint": key.public_key.fingerprint().to_string(),
    });
    let audit = NewAudit {
        operator: by.operator.clone(),
        action: AuditAction::Accept,
        target: AuditTarget::HostKey(device.id),
        client: by.client,
        change,
    };
    events::audit(&mut tx, by.at, audit).await?;
    tx.commit().await?;
    Ok(key)
}
