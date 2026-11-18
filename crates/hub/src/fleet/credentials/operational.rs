//! Operational keys: the credentials and runtime secrets a server of the operator needs, with
//! one active version each, and their retrieval by the daemon of an admitted server (R-147;
//! K-E5 Section 2; SEC-003).
//!
//! A value is sealed with the credential vault under the context
//! `credentials.operational_key_versions:<version id>` before it reaches storage, and leaves the
//! hub only through [`fetch`], to the daemon of a consumer. No metadata, audit entry, event,
//! error text or `Debug` output holds a value (LOG-007). Each change writes its rows, one audit
//! entry and one event in the caller's transaction. The time is a parameter (CQ-021).
//!
//! A new value makes the next version active and supersedes the old one at once; `revoke` ends
//! the active version with no new one; a key past its expiry is given no more and its version
//! reads as expired. A version that ended is never active again, and [`forget_ended`] empties its
//! sealed value 30 days after it ended. A device's consumer rows go with the device.

mod lifecycle;
mod model;
mod retrieval;
mod rules;
mod sealing;
mod store;
mod trail;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

pub use model::{Consumer, Due, Ending, Grant, Metadata, OperationalKey, Version, VersionState};
pub use retrieval::{current_for, fetch, record_held};
pub use rules::{NAME_RULE, Problem, WORKLOAD_RULE};

use super::{Secret, Vault};
use crate::{
    fleet::events::{self, Trail},
    ids::OperationalKeyId,
};
use store::{
    NewVersion,
    consumers::{self, Granted},
    reads,
};

/// Why an operational key operation failed. Messages are fixed text: no value, no name and no
/// SQL (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the operational key store is unavailable")]
    Database(#[from] sqlx::Error),
    #[error("the audit log or the event log cannot be written")]
    Trail(#[from] events::Error),
    #[error("an operational key cannot be sealed or opened")]
    Sealing(#[source] super::Error),
    #[error("a field of the operational key is not valid")]
    Invalid(Vec<Problem>),
    #[error("an operational key has this name already")]
    NameTaken,
    #[error("no operational key has this name")]
    UnknownKey,
    #[error("the device does not exist")]
    UnknownDevice,
    #[error("the key is not granted to this device for this workload")]
    NoGrant,
}

/// A key to make: its name, its metadata and its first value. No `Debug`: it holds the value.
pub struct NewKey {
    pub name: String,
    pub metadata: Metadata,
    pub value: Secret,
}

/// The keys that are not removed, by name, as they read at `now`.
///
/// # Errors
/// `Database` when the read fails.
pub async fn keys(db: &PgPool, now: DateTime<Utc>) -> Result<Vec<OperationalKey>, Error> {
    let mut conn = db.acquire().await?;
    Ok(reads::keys(&mut conn, None, now).await?)
}

/// The key named `name` that is not removed, as it reads at `now`.
///
/// # Errors
/// `Database` when the read fails.
pub async fn key(
    db: &PgPool,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Option<OperationalKey>, Error> {
    let mut conn = db.acquire().await?;
    Ok(reads::keys(&mut conn, Some(name), now).await?.pop())
}

/// Makes the key with its value as version 1, in the caller's transaction.
///
/// # Errors
/// `Invalid`, `NameTaken` (a removed key keeps its name), `Sealing`, `Database`, `Trail`.
pub async fn create(
    conn: &mut PgConnection,
    vault: &Vault,
    new: NewKey,
    trail: Trail<'_>,
) -> Result<OperationalKey, Error> {
    let mut problems = Vec::new();
    if !rules::is_name(&new.name) {
        problems.push(Problem {
            field: "name",
            message: NAME_RULE,
        });
    }
    problems.extend(rules::problems(&new.metadata));
    problems.extend(rules::value_problem(new.value.expose().len()));
    if !problems.is_empty() {
        return Err(Error::Invalid(problems));
    }
    let (now, by) = (trail.now, trail.operator().to_string());
    let id: OperationalKeyId = store::new_id();
    if !store::insert_key(conn, (id, &new.name), &new.metadata, (now, &by)).await? {
        return Err(Error::NameTaken);
    }
    add_version(conn, vault, (id, new.value, false), (now, &by)).await?;
    trail
        .write(conn, trail::created(id, &new.name, &new.metadata))
        .await?;
    read_back(conn, &new.name, now).await
}

/// Replaces the metadata of the key, in the caller's transaction.
///
/// # Errors
/// `UnknownKey`, `Invalid`, `Database`, `Trail`.
pub async fn update(
    conn: &mut PgConnection,
    name: &str,
    metadata: Metadata,
    trail: Trail<'_>,
) -> Result<OperationalKey, Error> {
    let problems = rules::problems(&metadata);
    if !problems.is_empty() {
        return Err(Error::Invalid(problems));
    }
    let id = locked(conn, name).await?;
    let now = trail.now;
    // An expiry already passed ends the version before a later expiry could revive it.
    store::settle_expired(conn, Some(id), now).await?;
    store::update_metadata(conn, id, &metadata).await?;
    trail
        .write(conn, trail::updated(id, name, &metadata))
        .await?;
    read_back(conn, name, now).await
}

/// Makes `value` the active version and supersedes the old one at once (R-147), in the
/// caller's transaction. `rotated_at_provider` records what the operator said; the hub changes
/// nothing at the provider.
///
/// # Errors
/// `UnknownKey`, `Invalid` for a value of a length a key cannot have, `Sealing`, `Database`,
/// `Trail`.
pub async fn rotate(
    conn: &mut PgConnection,
    vault: &Vault,
    (name, value, rotated_at_provider): (&str, Secret, bool),
    trail: Trail<'_>,
) -> Result<Ending, Error> {
    if let Some(problem) = rules::value_problem(value.expose().len()) {
        return Err(Error::Invalid(vec![problem]));
    }
    let id = locked(conn, name).await?;
    let (now, by) = (trail.now, trail.operator().to_string());
    store::settle_expired(conn, Some(id), now).await?;
    let ended = store::end_active(conn, id, (VersionState::Superseded, now)).await?;
    let made = add_version(conn, vault, (id, value, rotated_at_provider), (now, &by)).await?;
    let entry = trail::rotated((id, name), (ended, made), rotated_at_provider);
    trail.write(conn, entry).await?;
    ending(conn, (id, name, ended), now).await
}

/// Ends the active version with no new one, in the caller's transaction.
///
/// # Errors
/// `UnknownKey`, `Database`, `Trail`.
pub async fn revoke(
    conn: &mut PgConnection,
    name: &str,
    trail: Trail<'_>,
) -> Result<Ending, Error> {
    let id = locked(conn, name).await?;
    let now = trail.now;
    store::settle_expired(conn, Some(id), now).await?;
    let ended = store::end_active(conn, id, (VersionState::Revoked, now)).await?;
    let holders = holders_of(conn, id, ended).await?;
    trail
        .write(conn, trail::revoked((id, name), ended, holders.len()))
        .await?;
    ending(conn, (id, name, ended), now).await
}

/// Removes the key: ends its active version and its retrieval, in the caller's transaction.
/// Its name stays taken. The answer holds the key as it was at the removal.
///
/// # Errors
/// `UnknownKey`, `Database`, `Trail`.
pub async fn remove(
    conn: &mut PgConnection,
    name: &str,
    trail: Trail<'_>,
) -> Result<Ending, Error> {
    let id = locked(conn, name).await?;
    let now = trail.now;
    store::settle_expired(conn, Some(id), now).await?;
    let ended = store::end_active(conn, id, (VersionState::Revoked, now)).await?;
    let ending = ending(conn, (id, name, ended), now).await?;
    store::mark_removed(conn, id, now).await?;
    let entry = trail::removed((id, name), ended, ending.still_held_by.len());
    trail.write(conn, entry).await?;
    Ok(ending)
}

/// Lets the daemon of `grant.device` fetch the key for `grant.workload` (a trust decision of the
/// operator), in the caller's transaction. A grant that exists writes nothing.
///
/// # Errors
/// `UnknownKey`, `Invalid` for a workload that is not a label, `UnknownDevice`, `Database`,
/// `Trail`.
pub async fn grant(
    conn: &mut PgConnection,
    grant: Grant,
    trail: Trail<'_>,
) -> Result<OperationalKey, Error> {
    workload_fits(&grant.workload)?;
    let id = locked(conn, &grant.name).await?;
    let (now, by) = (trail.now, trail.operator().to_string());
    let row = (id, grant.device, grant.workload.as_str());
    match consumers::insert(conn, row, (now, &by)).await? {
        Granted::Made => trail.write(conn, trail::granted(id, &grant)).await?,
        Granted::Kept => {}
        Granted::NoDevice => return Err(Error::UnknownDevice),
    }
    read_back(conn, &grant.name, now).await
}

/// Ends a grant, in the caller's transaction. The daemon drops the key at its next report.
///
/// # Errors
/// `UnknownKey`, `NoGrant`, `Database`, `Trail`.
pub async fn ungrant(
    conn: &mut PgConnection,
    grant: Grant,
    trail: Trail<'_>,
) -> Result<OperationalKey, Error> {
    let id = locked(conn, &grant.name).await?;
    let now = trail.now;
    if !consumers::delete(conn, (id, grant.device, grant.workload.as_str())).await? {
        return Err(Error::NoGrant);
    }
    trail.write(conn, trail::ungranted(id, &grant)).await?;
    read_back(conn, &grant.name, now).await
}

/// Ends the versions of the keys past their expiry at `now`, then empties the sealed value of
/// each version that ended more than 30 days before `now`; how many it emptied. The daily
/// retention run calls it.
///
/// # Errors
/// `Database` when a write fails.
pub async fn forget_ended(db: &PgPool, now: DateTime<Utc>) -> Result<u64, Error> {
    let mut tx = db.begin().await?;
    store::settle_expired(&mut tx, None, now).await?;
    let emptied = store::forget_ended(&mut tx, lifecycle::forget_before(now)).await?;
    tx.commit().await?;
    Ok(emptied)
}

fn workload_fits(workload: &str) -> Result<(), Error> {
    if rules::is_workload(workload) {
        Ok(())
    } else {
        Err(Error::Invalid(vec![Problem {
            field: "workload",
            message: WORKLOAD_RULE,
        }]))
    }
}

async fn locked(conn: &mut PgConnection, name: &str) -> Result<OperationalKeyId, Error> {
    store::lock_key(conn, name).await?.ok_or(Error::UnknownKey)
}

async fn read_back(
    conn: &mut PgConnection,
    name: &str,
    now: DateTime<Utc>,
) -> Result<OperationalKey, Error> {
    reads::keys(conn, Some(name), now)
        .await?
        .pop()
        .ok_or(Error::UnknownKey)
}

/// Seals `value` for a new version of the key and stores it as the active one; its number.
async fn add_version(
    conn: &mut PgConnection,
    vault: &Vault,
    (key, value, rotated_at_provider): (OperationalKeyId, Secret, bool),
    made: store::Made<'_>,
) -> Result<i32, Error> {
    let id = store::new_id();
    let sealed = sealing::seal(vault, id, &value).map_err(Error::Sealing)?;
    drop(value);
    let version = NewVersion {
        id,
        key,
        sealed: &sealed,
        rotated_at_provider,
    };
    Ok(store::insert_version(conn, version, made).await?)
}

async fn holders_of(
    conn: &mut PgConnection,
    key: OperationalKeyId,
    ended: Option<i32>,
) -> Result<Vec<Consumer>, Error> {
    match ended {
        Some(number) => Ok(reads::holders(conn, key, number).await?),
        None => Ok(Vec::new()),
    }
}

async fn ending(
    conn: &mut PgConnection,
    (key, name, ended): (OperationalKeyId, &str, Option<i32>),
    now: DateTime<Utc>,
) -> Result<Ending, Error> {
    Ok(Ending {
        key: read_back(conn, name, now).await?,
        ended,
        still_held_by: holders_of(conn, key, ended).await?,
    })
}
