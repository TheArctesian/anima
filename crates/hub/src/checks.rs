//! Checks: what each device is checked for, the results, the status they give and the
//! uptime they add up to (spec 6.4).
//!
//! Other domains use only the K3 functions: [`statuses`], [`uptimes`], [`for_device`],
//! [`for_devices`], [`add_default_check`], [`device_status`], [`resolved_ips`] and [`evaluate`]. The writes,
//! [`record_beat`], [`due_checks`] and the retention functions serve this domain's GraphQL,
//! push route and scheduler; they are public so the integration tests can drive them.
//!
//! HTTP secrets are sealed with the vault, bound to their check, and never returned by a
//! read; only [`load_http_secrets`] opens them, for the probe (SEC-003, LOG-007).

pub mod config;
mod evaluation;
pub mod graphql;
mod model;
pub mod probe;
mod push;
mod rules;
mod status;
mod store;
#[cfg(test)]
mod tests;
mod uptime;

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Connection as _, PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    credentials::{self, Vault},
    devices,
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
};

use config::{CheckConfig, HttpSecrets, IpVersion};
use rules::Prepared;

pub use evaluation::{CheckSettings, CheckState, evaluate};
pub use model::{
    Beat, Check, CheckChange, Definition, Due, FieldError, Heartbeat, Missing, NewCheck, PushToken,
};
pub use push::push_router;
pub use store::Trimmed;
pub use uptime::UptimeWindow;

/// Why a checks operation failed. Messages hold no SQL, no input value and no secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// The sealed HTTP secrets cannot be opened; the source names no key or plaintext.
    #[error("the check secrets cannot be used")]
    Credentials(#[from] credentials::Error),
    /// The stored configuration is not in the form this version reads.
    #[error("the stored configuration of check {0} cannot be read")]
    Unreadable(CheckId),
    #[error("the devices of the check cannot be read")]
    Devices(#[source] devices::Error),
    #[error("the maintenance windows cannot be read")]
    Maintenance(#[source] maintenance::Error),
}

/// The names of the fields in error, never their values.
fn field_names(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|error| error.field)
        .collect::<Vec<_>>()
        .join(", ")
}

fn from_devices(error: devices::Error) -> Error {
    match error {
        devices::Error::NotFound(devices::Missing::Device(id)) => {
            Error::NotFound(Missing::Device(id))
        }
        other => Error::Devices(other),
    }
}

/// The status of each device: the worst of its checks, with pause, groups and maintenance
/// applied (CHK-004, CHK-006). Three queries however many devices (API-005).
///
/// # Errors
/// `Database`, `Devices` or `Maintenance` when a read fails.
pub async fn statuses(
    db: &PgPool,
    devices: &[DeviceId],
    now: DateTime<Utc>,
) -> Result<HashMap<DeviceId, Status>, Error> {
    let checks = store::standings(db, devices).await?;
    let known = devices::list(db).await.map_err(from_devices)?;
    let active = maintenance::active_targets(db, now)
        .await
        .map_err(Error::Maintenance)?;
    Ok(status::by_device(devices, &checks, &known, &active))
}

/// The status of one device (CHK-004).
///
/// # Errors
/// `NotFound` when there is no such device.
pub async fn device_status(
    db: &PgPool,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<Status, Error> {
    let known = devices::get(db, device).await.map_err(from_devices)?;
    let checks = store::standings(db, &[device]).await?;
    let active = maintenance::active_targets(db, now)
        .await
        .map_err(Error::Maintenance)?;
    let mut shown = status::by_device(&[device], &checks, &[known], &active);
    Ok(shown.remove(&device).unwrap_or(Status::Unknown))
}

/// The uptime of each device over the window ending at `now`, from the aggregate tables
/// (CHK-011). A device with no counted beats is absent.
///
/// # Errors
/// `Database` when the query fails.
pub async fn uptimes(
    db: &PgPool,
    devices: &[DeviceId],
    window: UptimeWindow,
    now: DateTime<Utc>,
) -> Result<HashMap<DeviceId, f64>, Error> {
    let span = (window.start(now), now);
    let tallies = store::tallies(db, devices, window.resolution(), span).await?;
    Ok(tallies
        .into_iter()
        .filter_map(|(device, tally)| uptime::ratio(tally).map(|ratio| (device, ratio)))
        .collect())
}

/// The device's checks in byte order of the name.
///
/// # Errors
/// `Unreadable` when a stored configuration cannot be read.
pub async fn for_device(db: &PgPool, device: DeviceId) -> Result<Vec<Check>, Error> {
    store::by_devices(db, &[device]).await
}

/// The checks of each device, each device's in byte order of the name, in one query
/// (API-005). A device with no checks is absent.
///
/// # Errors
/// `Unreadable` when a stored configuration cannot be read.
pub async fn for_devices(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<Check>>, Error> {
    store::grouped_by_devices(db, devices).await
}

/// The check with this id.
///
/// # Errors
/// `NotFound` when there is no such check.
pub async fn get(db: &PgPool, id: CheckId) -> Result<Check, Error> {
    store::fetch(db, id).await
}

/// The status the check shows: its stored status with pause, group and maintenance applied
/// (CHK-004, CHK-006). It agrees with [`device_status`] for the check's device.
///
/// # Errors
/// `NotFound` when the device is gone; `Database`, `Devices` or `Maintenance` when a read fails.
pub async fn check_status(db: &PgPool, check: &Check, now: DateTime<Utc>) -> Result<Status, Error> {
    let device = devices::get(db, check.device).await.map_err(from_devices)?;
    let checks = store::standings(db, &[check.device]).await?;
    let active = maintenance::active_targets(db, now)
        .await
        .map_err(Error::Maintenance)?;
    Ok(status::of_check(check.id, &device, &checks, &active))
}

/// The uptime of one check over the window ending at `now`, from the aggregate tables;
/// `None` when it has no counted beats (CHK-011).
///
/// # Errors
/// `Database` when the query fails.
pub async fn check_uptime(
    db: &PgPool,
    check: CheckId,
    window: UptimeWindow,
    now: DateTime<Utc>,
) -> Result<Option<f64>, Error> {
    let span = (window.start(now), now);
    let tally = store::check_tally(db, check, window.resolution(), span).await?;
    Ok(uptime::ratio(tally))
}

/// Up to `limit` heartbeats of the check recorded before `before`, newest first.
///
/// # Errors
/// `Database` when the query fails.
pub async fn heartbeats(
    db: &PgPool,
    check: CheckId,
    before: Option<DateTime<Utc>>,
    limit: u32,
) -> Result<Vec<Heartbeat>, Error> {
    store::heartbeats_before(db, check, before, i64::from(limit)).await
}

/// The PUSH check that holds this token, or `None` (R-028).
///
/// # Errors
/// `Database` when the query fails; `Unreadable` when the stored configuration is not.
pub async fn push_check(db: &PgPool, token: &str) -> Result<Option<Check>, Error> {
    store::by_push_token(db, token).await
}

/// For each device, the address that its check with the latest beat resolved (R-028).
///
/// # Errors
/// `Database` when the query fails.
pub async fn resolved_ips(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, IpAddr>, Error> {
    store::resolved_ips(db, devices).await
}

/// Adds the default check, an ICMP check named "Ping" with the contract defaults, in the
/// caller's transaction (spec 6.2).
///
/// # Errors
/// `Validation` (`deviceId`) when there is no such device.
pub async fn add_default_check(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<CheckId, Error> {
    let id = CheckId::from(Uuid::now_v7());
    let config = CheckConfig::Icmp {
        timeout_ms: 2000,
        ip_version: IpVersion::V4,
    };
    let row = store::Insert {
        device,
        parent: None,
        name: "Ping",
        config: &config,
        settings: CheckSettings::default(),
        push_token: None,
        sealed: None,
    };
    Ok(store::insert(conn, id, row).await?.id)
}

fn sealed(vault: &Vault, id: CheckId, prepared: &Prepared) -> Option<Vec<u8>> {
    prepared
        .secrets
        .as_ref()
        .map(|secrets| store::seal(vault, id, secrets))
}

/// Adds a check in the caller's transaction. A parent must be a group on the same device.
///
/// # Errors
/// `Validation` with every field in error, including the constraint the database refused.
pub async fn create_check(
    conn: &mut PgConnection,
    vault: &Vault,
    new: NewCheck,
) -> Result<Check, Error> {
    let id = CheckId::from(Uuid::now_v7());
    let (name, prepared) = match (rules::name(&new.name), rules::prepare(new.definition, None)) {
        (Ok(name), Ok(prepared)) => (name, prepared),
        (name, prepared) => {
            let errors = [name.err(), prepared.err()].into_iter().flatten().collect();
            return Err(Error::Validation(errors));
        }
    };
    let row = store::Insert {
        device: new.device,
        parent: new.parent,
        name: &name,
        config: &prepared.config,
        settings: new.settings,
        push_token: prepared.push_token.as_ref(),
        sealed: sealed(vault, id, &prepared),
    };
    store::insert(conn, id, row).await
}

/// Changes the fields present in the caller's transaction (see [`CheckChange`] for the
/// secret rule). The type of a check never changes.
///
/// # Errors
/// `NotFound` when there is no such check; `Validation` with the fields in error.
pub async fn update_check(
    conn: &mut PgConnection,
    vault: &Vault,
    id: CheckId,
    mut change: CheckChange,
) -> Result<Check, Error> {
    let definition = change.definition.take();
    let name = change.name.as_deref().map(rules::name).transpose();
    let mut tx = conn.begin().await?;
    let current = store::lock(&mut tx, id).await?;
    let prepared = match definition {
        Some(definition) => prepare_change(vault, id, current, definition).map(Some),
        None => Ok(None),
    };
    let (name, prepared) = match (name, prepared) {
        (Ok(name), Ok(prepared)) => (name, prepared),
        (name, prepared) => {
            let errors = [name.err(), prepared.err()].into_iter().flatten().collect();
            return Err(Error::Validation(errors));
        }
    };
    let config = prepared
        .as_ref()
        .map(|prepared| (&prepared.config, sealed(vault, id, prepared)));
    let changed = store::Change {
        fields: &change,
        name: name.as_deref(),
        config,
        push_token: prepared.as_ref().and_then(|p| p.push_token.as_ref()),
    };
    let check = store::update(&mut tx, id, changed).await?;
    tx.commit().await?;
    Ok(check)
}

/// Prepares a new definition of the same type, filling empty secrets from the stored ones.
/// Stored secrets that do not open for this check give a field error that asks for every
/// secret again.
fn prepare_change(
    vault: &Vault,
    id: CheckId,
    current: store::Current,
    definition: Definition,
) -> Result<Prepared, FieldError> {
    if definition.kind().as_sql() != current.kind {
        return Err(FieldError::new(
            "config",
            "A check keeps its type. Delete it and add a new check instead.",
        ));
    }
    let stored = match &definition {
        Definition::Http(input) if rules::needs_stored(input) => {
            current.http(vault, id).map_err(|_| {
                FieldError::new(
                    "config",
                    "The stored secrets cannot be opened. Enter every secret again.",
                )
            })?
        }
        _ => None,
    };
    rules::prepare(definition, stored)
}

/// Deletes the check in the caller's transaction; its children leave the group.
///
/// # Errors
/// `NotFound` when there is no such check.
pub async fn delete_check(conn: &mut PgConnection, id: CheckId) -> Result<Check, Error> {
    store::delete(conn, id).await
}

/// Pauses or resumes the check (CHK-005); either way its state starts over.
///
/// # Errors
/// `NotFound` when there is no such check.
pub async fn set_paused(
    conn: &mut PgConnection,
    id: CheckId,
    paused: bool,
) -> Result<Check, Error> {
    store::set_paused(conn, id, paused).await
}

/// The opened HTTP secrets of the check, for its probe only; `None` when it has none.
///
/// # Errors
/// `Credentials` when the sealed value does not open for this check.
pub async fn load_http_secrets(
    db: &PgPool,
    vault: &Vault,
    check: CheckId,
) -> Result<Option<HttpSecrets>, Error> {
    store::load_http_secrets(db, vault, check).await
}

/// Records one result in the caller's transaction: the heartbeat, its minute, hour and day
/// aggregates, and the check's new state (CHK-003, DB-008).
///
/// # Errors
/// `NotFound` when the check no longer exists; nothing is written then.
pub async fn record_beat(
    conn: &mut PgConnection,
    check: CheckId,
    beat: &Beat,
) -> Result<(), Error> {
    store::record_beat(conn, check, beat).await
}

/// Up to `limit` checks that are not paused and are due at `now` (CHK-001).
///
/// # Errors
/// `Database` when the query fails.
pub async fn due_checks(db: &PgPool, now: DateTime<Utc>, limit: u32) -> Result<Vec<Due>, Error> {
    store::due(db, now, limit).await
}

/// Deletes the heartbeats of checks that no longer exist (R-004).
///
/// # Errors
/// `Database` when the query fails.
pub async fn delete_orphan_heartbeats(db: &PgPool) -> Result<u64, Error> {
    store::delete_orphan_heartbeats(db).await
}

/// Deletes aggregate rows past their retention (DB-009).
///
/// # Errors
/// `Database` when a query fails.
pub async fn trim_aggregates(db: &PgPool, now: DateTime<Utc>) -> Result<Trimmed, Error> {
    store::trim_aggregates(db, now).await
}

/// Drops the heartbeat partitions of the UTC days before `cutoff` (DB-005).
///
/// # Errors
/// `Database` when the cutoff is inside the 30-day retention.
pub async fn drop_heartbeat_days_before(
    db: &PgPool,
    cutoff: NaiveDate,
    now: DateTime<Utc>,
) -> Result<Vec<String>, Error> {
    store::drop_heartbeat_days_before(db, cutoff, now).await
}

/// Creates the heartbeat partitions of today and the next `days` days (DB-004).
///
/// # Errors
/// `Database` when a day is outside the window the database accepts.
pub async fn create_days_ahead(db: &PgPool, today: DateTime<Utc>, days: u32) -> Result<(), Error> {
    store::create_days_ahead(db, today, days).await
}
