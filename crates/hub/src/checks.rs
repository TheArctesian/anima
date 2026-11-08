//! Checks: what each device is checked for, the results, the status they give and the
//! uptime they add up to (spec 6.4). [`spawn_scheduler`] runs the checks (HUB-003).
//!
//! Other domains use only the contract functions: [`statuses`], [`uptimes`], [`for_device`],
//! [`add_default_check`], [`device_status`], [`resolved_ips`], [`http_urls`] and [`evaluate`].
//! The writes, [`record_beat`], [`due_checks`] and the retention functions serve this domain's
//! GraphQL, push route and scheduler; they are public so the integration tests can drive them.
//! [`exposition_router`] serves `GET /metrics` (INT-001).
//!
//! HTTP and SNMP secrets are sealed with the vault, bound to their check, and never returned
//! by a read; only [`load_http_secrets`] and [`load_snmp_secrets`] open them, for the probe
//! (SEC-003, LOG-007).

pub mod config;
mod evaluation;
mod exposition;
pub mod graphql;
mod model;
pub mod probe;
mod push;
mod rules;
mod scheduler;
mod sealing;
mod status;
mod store;
#[cfg(test)]
mod tests;
mod uptime;

use std::{collections::HashMap, net::IpAddr};

use chrono::{DateTime, Days, Utc};
use sqlx::{Connection as _, PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    api_errors::field_names,
    credentials::{self, Vault},
    devices,
    ids::{CheckId, DeviceId},
    maintenance,
    status::Status,
};

use config::{CheckConfig, IpVersion};
use sealing::{prepare_change, sealed};

pub use evaluation::{CheckSettings, CheckState, evaluate};
pub use exposition::exposition_router;
pub(in crate::checks) use model::Attempt;
pub use model::{
    Beat, Check, CheckChange, CheckDefaults, Definition, Due, FieldError, Heartbeat, HttpSetup,
    Missing, NewCheck, PushToken, Recorded, SnmpSetup,
};
pub use push::push_router;
pub use scheduler::{Probing, spawn_scheduler};
pub use uptime::UptimeWindow;

/// Days after today whose heartbeat partitions [`drop_expired`] creates (DB-004).
const PARTITION_DAYS_AHEAD: u32 = 7;

/// Why a checks operation failed. Messages hold no SQL, no input value and no secret.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// The sealed secrets cannot be opened; the source names no key or plaintext.
    #[error("the check secrets cannot be used")]
    Credentials(#[from] credentials::Error),
    /// The stored configuration is not in the form this version reads.
    #[error("the stored configuration of check {0} cannot be read")]
    Unreadable(CheckId),
    /// The sealed secrets were given for another origin or other secret headers (HTTP), or
    /// another version or user name (SNMP), than the stored configuration names, so they are
    /// not used (SEC-005).
    #[error("the stored secrets do not match the check's address")]
    SecretsUnbound,
    /// Another check already has this push token. The database detail, which holds the
    /// token, is dropped (LOG-007).
    #[error("the push token is already in use")]
    PushTokenTaken,
    #[error("the devices of the check cannot be read")]
    Devices(#[source] devices::Error),
    #[error("the maintenance windows cannot be read")]
    Maintenance(#[source] maintenance::Error),
    /// The OS random source failed, so no push token was made.
    #[error("the OS random source failed")]
    Random(#[source] getrandom::Error),
}

impl Error {
    /// For the process log: what failed, as a fixed word, never the source text (LOG-007,
    /// API-010).
    #[must_use]
    pub const fn fault(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            _ => "checks",
        }
    }
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

/// The id and URL of each device's HTTP checks, each device's in byte order of the name then
/// id, in one query (API-005, WEB-002). A device with no HTTP check is absent. Other check
/// types' configurations are not read.
///
/// # Errors
/// `Database` when the query fails.
pub async fn http_urls(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, Vec<(CheckId, String)>>, Error> {
    Ok(store::http_urls(db, devices).await?)
}

/// The check with this id.
///
/// # Errors
/// `NotFound` when there is no such check.
pub async fn get(db: &PgPool, id: CheckId) -> Result<Check, Error> {
    store::fetch(db, id).await
}

/// The checks with these ids, in one query (API-005). An id with no check is absent.
///
/// # Errors
/// `Unreadable` when a stored configuration cannot be read.
pub async fn get_many(db: &PgPool, ids: &[CheckId]) -> Result<Vec<Check>, Error> {
    store::fetch_many(db, ids).await
}

/// For each device, the address that its check with the latest beat resolved.
///
/// # Errors
/// `Database` when the query fails.
pub async fn resolved_ips(
    db: &PgPool,
    devices: &[DeviceId],
) -> Result<HashMap<DeviceId, IpAddr>, Error> {
    store::resolved_ips(db, devices).await
}

/// The settings a new check starts with (D-26, DB-011).
///
/// # Errors
/// `Database` when the read fails.
pub async fn defaults(db: &PgPool) -> Result<CheckDefaults, Error> {
    store::read_defaults(db).await
}

/// Replaces the stored defaults in the caller's transaction.
///
/// # Errors
/// `Validation` naming the first field outside the bounds of `checks.checks` (the database
/// backstop); the GraphQL input lists every such field before it gets here (API-010).
pub async fn set_defaults(
    conn: &mut PgConnection,
    new: CheckDefaults,
) -> Result<CheckDefaults, Error> {
    store::write_defaults(conn, new).await
}

/// Adds the default check, an ICMP check named "Ping" with the stored defaults, in the
/// caller's transaction (spec 6.2, D-26).
///
/// # Errors
/// `Validation` (`deviceId`) when there is no such device.
pub async fn add_default_check(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<CheckId, Error> {
    let id = CheckId::from(Uuid::now_v7());
    let settings = store::read_defaults(&mut *conn).await?.settings();
    let config = CheckConfig::Icmp {
        timeout_ms: 2000,
        ip_version: IpVersion::V4,
    };
    let row = store::Insert {
        device,
        parent: None,
        name: "Ping",
        config: &config,
        settings,
        push_token: None,
        sealed: None,
    };
    Ok(store::insert(conn, id, row).await?.id)
}

/// A new push token from 32 bytes of the operating system's random source: the I/O edge of
/// token making.
///
/// # Errors
/// `Random` when the OS random source fails.
pub(in crate::checks) fn fresh_push_token() -> Result<PushToken, Error> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(PushToken::from_random(&bytes))
}

/// Adds a check in the caller's transaction. A parent must be a group on the same device.
///
/// # Errors
/// `Validation` with every field in error, including the constraint the database refused;
/// `Credentials` (`Random`) when the OS random source fails while sealing secrets.
pub async fn create_check(
    conn: &mut PgConnection,
    vault: &Vault,
    new: NewCheck,
) -> Result<Check, Error> {
    let id = CheckId::from(Uuid::now_v7());
    let prepared = rules::prepare(new.definition, rules::Stored::Nothing);
    let (name, prepared) = match (rules::name(&new.name), prepared) {
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
        sealed: sealed(vault, id, &prepared)?,
    };
    store::insert(conn, id, row).await
}

/// Changes the fields present in the caller's transaction (see [`CheckChange`] for the
/// secret rule). The type of a check never changes.
///
/// # Errors
/// `NotFound` when there is no such check; `Validation` with the fields in error;
/// `Credentials` (`Random`) when the OS random source fails while sealing secrets.
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
        .map(|prepared| Ok::<_, Error>((&prepared.config, sealed(vault, id, prepared)?)))
        .transpose()?;
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

/// The stored HTTP configuration of the check with its opened secrets, for its probe only:
/// the probe sends this configuration, so the secrets go only where they were given for
/// (SEC-005). `None` when the check is not an HTTP check and holds no secrets.
///
/// # Errors
/// `Credentials` when the sealed value does not open for this check; `SecretsUnbound` when
/// the configuration no longer names the destination they were given for; `Unreadable` when
/// the check holds secrets but no readable HTTP configuration.
pub async fn load_http_secrets(
    db: &PgPool,
    vault: &Vault,
    check: CheckId,
) -> Result<Option<HttpSetup>, Error> {
    store::load_http_secrets(db, vault, check).await
}

/// The stored SNMP configuration of the check with its opened secrets, for its probe only:
/// the secrets go only to the version and user name they were given for (SEC-005). `None`
/// when the check is not an SNMP check and holds no secrets.
///
/// # Errors
/// `Credentials` when the sealed value does not open for this check; `SecretsUnbound` when
/// the configuration names another version or user name; `Unreadable` when the check holds
/// secrets but no readable SNMP configuration.
pub async fn load_snmp_secrets(
    db: &PgPool,
    vault: &Vault,
    check: CheckId,
) -> Result<Option<SnmpSetup>, Error> {
    store::load_snmp_secrets(db, vault, check).await
}

/// Records one result in the caller's transaction: the heartbeat, its minute, hour and day
/// aggregates, and the check's new state (CHK-003, DB-008). A paused check, or a beat not
/// newer than the check's last beat, writes nothing and gives [`Recorded::Skipped`] (CHK-005).
///
/// # Errors
/// `NotFound` when the check no longer exists; nothing is written then.
pub async fn record_beat(
    conn: &mut PgConnection,
    check: CheckId,
    beat: &Beat,
) -> Result<Recorded, Error> {
    store::record_beat(conn, check, beat).await
}

/// Records one attempt in the caller's transaction: locks the check, evaluates the result on
/// its current state and settings, so a beat, pause or change that landed meanwhile applies,
/// and writes the beat with [`record_beat`] (CHK-002, CHK-005). Returns the beat written, or
/// `None` when the check is paused or already has a beat at least as new.
///
/// # Errors
/// `NotFound` when the check no longer exists.
pub(in crate::checks) async fn record_attempt(
    conn: &mut PgConnection,
    check: CheckId,
    attempt: &Attempt<'_>,
) -> Result<Option<Beat>, Error> {
    store::record_attempt(conn, check, attempt).await
}

/// Up to `limit` checks that are not paused and are due at `now` (CHK-001). PUSH checks are
/// included: when one is due, the push did not come in its interval and the scheduler records
/// that. Groups are never due.
///
/// # Errors
/// `Database` when the query fails.
pub async fn due_checks(db: &PgPool, now: DateTime<Utc>, limit: u32) -> Result<Vec<Due>, Error> {
    store::due(db, now, limit).await
}

/// The daily retention run of checks, passed the operator's period (D-25, HUB-005): creates the
/// heartbeat partitions of today and the week after it (DB-004), drops those older than
/// `heartbeats_keep` (DB-005), deletes the aggregate rows past their fixed periods (DB-009) and
/// the heartbeats of deleted checks (DEV-015). The database refuses to drop a heartbeat
/// partition less than 1 day old. A step that fails stops the run; `upkeep.rs` retries it.
///
/// # Errors
/// `Database` when a step fails, or when the database refuses the cutoff.
pub async fn drop_expired(
    db: &PgPool,
    now: DateTime<Utc>,
    heartbeats_keep: Days,
) -> Result<(), Error> {
    store::create_days_ahead(db, now, PARTITION_DAYS_AHEAD).await?;
    let dropped = store::drop_heartbeat_days_before(db, now, heartbeats_keep).await?;
    if !dropped.is_empty() {
        tracing::info!(domain = "checks", dropped = ?dropped, "old heartbeat partitions dropped");
    }
    store::trim_aggregates(db, now).await?;
    store::delete_orphan_heartbeats(db).await?;
    Ok(())
}
