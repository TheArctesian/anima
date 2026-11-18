//! Retrieval by the daemon of a consumer: the fetch of one key, the versions a device may get,
//! and what its cache holds (K-E5 Section 2.1, R-147). The metrics ingest calls these three.

use chrono::{DateTime, Utc};
use daemon_core::wire::{FetchedKey, KeyValue, KeyVersion};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{Error, Vault, sealing, store, store::consumers, trail};
use crate::{
    fleet::{
        credentials,
        events::{self, AuditAction, AuditTarget, ClientAddr, NewAudit, Operator},
    },
    ids::DeviceId,
};

/// The active version of the key named `name` for the daemon of `device` at `now`, with one
/// audit entry and the fetch recorded on its consumer rows; `None` when the key does not exist,
/// has no active version, or `device` is no consumer of it. The three cases are one answer
/// and take one path (K-E5 Section 2.1).
///
/// # Errors
/// `Sealing` when the stored value does not open, `Database`, `Trail`.
pub async fn fetch(
    db: &PgPool,
    vault: &Vault,
    (device, name): (DeviceId, &str),
    (now, client): (DateTime<Utc>, ClientAddr),
) -> Result<Option<FetchedKey>, Error> {
    let mut tx = db.begin().await?;
    let Some(found) = store::fetchable(&mut tx, (device, name), now).await? else {
        return Ok(None);
    };
    let secret = sealing::open(vault, found.version, &found.sealed).map_err(Error::Sealing)?;
    let value = KeyValue::new(Zeroizing::new(secret.expose().to_vec()))
        .ok_or(Error::Sealing(credentials::Error::Malformed))?;
    drop(secret);
    consumers::mark_fetched(&mut tx, (found.key, device), (found.number, now)).await?;
    let entry = NewAudit {
        operator: Operator::hub(),
        action: AuditAction::Fetch,
        target: AuditTarget::OperationalKey(found.key),
        client,
        change: trail::fetched(name, found.number, device),
    };
    events::audit(&mut tx, now, entry).await?;
    tx.commit().await?;
    Ok(Some(FetchedKey {
        name: name.to_owned(),
        version: version_number(found.number),
        value,
    }))
}

/// Each key the daemon of `device` may get at `now`, by name, with its active version (K-E5
/// Section 2.1, `current`).
///
/// # Errors
/// `Database` when the read fails.
pub async fn current_for(
    db: &PgPool,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<Vec<KeyVersion>, Error> {
    let mut conn = db.acquire().await?;
    let rows = consumers::current_for(&mut conn, device, now).await?;
    Ok(rows
        .into_iter()
        .map(|(name, number)| KeyVersion {
            name,
            version: version_number(number),
        })
        .collect())
}

/// Records the versions in the cache of the daemon of `device` at `now` on its consumer rows; a
/// key the daemon does not name holds no version (K-E5 Section 2.1, `held`).
///
/// # Errors
/// `Database` when the write fails.
pub async fn record_held(
    db: &PgPool,
    device: DeviceId,
    held: &[KeyVersion],
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let names: Vec<String> = held.iter().map(|one| one.name.clone()).collect();
    let versions: Vec<i32> = held
        .iter()
        .map(|one| i32::try_from(one.version).unwrap_or(i32::MAX))
        .collect();
    let mut conn = db.acquire().await?;
    consumers::record_held(&mut conn, device, (&names, &versions), now).await?;
    Ok(())
}

/// A version number as K8 carries it; the column's check keeps it at least 1.
fn version_number(number: i32) -> u32 {
    u32::try_from(number).unwrap_or_default()
}
