//! Check writes, each in the caller's transaction in a savepoint, and the sealed HTTP and
//! SNMP secrets (SEC-003).

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Connection as _, PgConnection, PgExecutor};

use super::{
    rows::{CheckRow, config_of, stored_config, write_error},
    sealed::open,
    snmp_sealed,
};
use crate::{
    checks::{
        Error,
        config::{CheckConfig, HttpSecrets},
        evaluation::CheckSettings,
        model::{Check, CheckChange, HttpSetup, Missing, PushToken, SnmpSetup},
        rules::{StoredHttp, StoredSnmp},
    },
    credentials::Vault,
    ids::{CheckId, DeviceId},
};

fn small(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A check row to insert, already validated and with its secrets sealed.
pub(in crate::checks) struct Insert<'a> {
    pub(in crate::checks) device: DeviceId,
    pub(in crate::checks) parent: Option<CheckId>,
    pub(in crate::checks) name: &'a str,
    pub(in crate::checks) config: &'a CheckConfig,
    pub(in crate::checks) settings: CheckSettings,
    pub(in crate::checks) push_token: Option<&'a PushToken>,
    pub(in crate::checks) sealed: Option<Vec<u8>>,
}

pub(in crate::checks) async fn insert(
    conn: &mut PgConnection,
    id: CheckId,
    row: Insert<'_>,
) -> Result<Check, Error> {
    let mut tx = conn.begin().await?;
    sqlx::query(
        "INSERT INTO checks.checks (id, device_id, parent_id, parent_type, type, name, \
                interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms, \
                invert, config, push_token, secrets_sealed) \
         VALUES ($1, $2, $3, CASE WHEN $3::uuid IS NULL THEN NULL ELSE 'group' END, $4, $5, \
                 $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(id)
    .bind(row.device)
    .bind(row.parent)
    .bind(row.config.kind().as_sql())
    .bind(row.name)
    .bind(small(row.settings.interval_seconds))
    .bind(i32::from(row.settings.retries))
    .bind(small(row.settings.retry_interval_seconds))
    .bind(small(row.settings.degraded_threshold_ms))
    .bind(row.settings.invert)
    .bind(stored_config(row.config)?)
    .bind(row.push_token.map(PushToken::expose))
    .bind(row.sealed)
    .execute(&mut *tx)
    .await
    .map_err(write_error)?;
    let check = super::fetch(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(check)
}

/// The stored type, configuration and sealed secrets of a check, locked for the change.
pub(in crate::checks) struct Current {
    pub(in crate::checks) kind: String,
    config: Value,
    sealed: Option<Vec<u8>>,
}

impl Current {
    /// The stored HTTP setup with its secrets opened, for the empty-secret rule.
    pub(in crate::checks) fn http(
        self,
        vault: &Vault,
        check: CheckId,
    ) -> Result<Option<StoredHttp>, Error> {
        let Some(CheckConfig::Http(config)) = config_of(&self.kind, self.config) else {
            return Ok(None);
        };
        let secrets = match self.sealed {
            Some(bytes) => open(vault, check, &config, &bytes)?,
            None => HttpSecrets::default(),
        };
        Ok(Some((config, secrets)))
    }

    /// The stored SNMP configuration with its secrets opened, for the empty-secret rule.
    /// `None` when the check is not an SNMP check or holds no secrets.
    pub(in crate::checks) fn snmp(
        self,
        vault: &Vault,
        check: CheckId,
    ) -> Result<Option<StoredSnmp>, Error> {
        let Some(CheckConfig::Snmp(config)) = config_of(&self.kind, self.config) else {
            return Ok(None);
        };
        self.sealed
            .map(|bytes| {
                let secrets = snmp_sealed::open(vault, check, &config, &bytes)?;
                Ok((config, secrets))
            })
            .transpose()
    }
}

pub(in crate::checks) async fn lock(
    conn: &mut PgConnection,
    id: CheckId,
) -> Result<Current, Error> {
    let row: Option<(String, Value, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT type, config, secrets_sealed FROM checks.checks WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    let (kind, config, sealed) = row.ok_or(Error::NotFound(Missing::Check(id)))?;
    Ok(Current {
        kind,
        config,
        sealed,
    })
}

/// A validated change: the name is checked, a new configuration is prepared and sealed.
pub(in crate::checks) struct Change<'a> {
    pub(in crate::checks) fields: &'a CheckChange,
    pub(in crate::checks) name: Option<&'a str>,
    /// The new configuration and its sealed secrets (`None` removes them).
    pub(in crate::checks) config: Option<(&'a CheckConfig, Option<Vec<u8>>)>,
    pub(in crate::checks) push_token: Option<&'a PushToken>,
}

/// Changes only the fields present, in the transaction that `lock` started.
pub(in crate::checks) async fn update(
    conn: &mut PgConnection,
    id: CheckId,
    change: Change<'_>,
) -> Result<Check, Error> {
    let fields = change.fields;
    let (config, sealed) = match change.config {
        Some((config, sealed)) => (Some(stored_config(config)?), sealed),
        None => (None, None),
    };
    sqlx::query(
        "UPDATE checks.checks SET \
            parent_id = CASE WHEN $2 THEN $3 ELSE parent_id END, \
            parent_type = CASE WHEN NOT $2 THEN parent_type \
                               WHEN $3::uuid IS NULL THEN NULL ELSE 'group' END, \
            name = COALESCE($4, name), config = COALESCE($5, config), \
            secrets_sealed = CASE WHEN $5::jsonb IS NULL THEN secrets_sealed ELSE $6 END, \
            push_token = COALESCE($7, push_token), \
            interval_seconds = COALESCE($8, interval_seconds), \
            retries = COALESCE($9, retries), \
            retry_interval_seconds = COALESCE($10, retry_interval_seconds), \
            degraded_threshold_ms = COALESCE($11, degraded_threshold_ms), \
            invert = COALESCE($12, invert), operator_edited = operator_edited OR $13, updated_at = now() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(fields.parent.is_some())
    .bind(fields.parent.flatten())
    .bind(change.name)
    .bind(config)
    .bind(sealed)
    .bind(change.push_token.map(PushToken::expose))
    .bind(fields.interval_seconds.map(small))
    .bind(fields.retries.map(i32::from))
    .bind(fields.retry_interval_seconds.map(small))
    .bind(fields.degraded_threshold_ms.map(small))
    .bind(fields.invert)
    .bind(!fields.regrouping)
    .execute(&mut *conn)
    .await
    .map_err(write_error)?;
    super::fetch(conn, id).await
}

/// Deletes the check; its children leave the group through the foreign key. Returns the
/// check as it was.
pub(in crate::checks) async fn delete(
    conn: &mut PgConnection,
    id: CheckId,
) -> Result<Check, Error> {
    let mut tx = conn.begin().await?;
    let row = sqlx::query_as::<_, CheckRow>(select_check!("WHERE id = $1 FOR UPDATE"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound(Missing::Check(id)))?;
    sqlx::query("DELETE FROM checks.checks WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Check::try_from(row)
}

/// Pauses or resumes the check (CHK-005). Either way its state starts over: pausing makes it
/// Unknown with no failures, and the next probe is due at once.
pub(in crate::checks) async fn set_paused(
    conn: &mut PgConnection,
    id: CheckId,
    paused: bool,
) -> Result<Check, Error> {
    let row = sqlx::query_as::<_, CheckRow>(concat!(
        "UPDATE checks.checks SET paused = $2, \
                status = CASE WHEN $2 THEN 'unknown' ELSE status END, \
                failures = CASE WHEN $2 THEN 0 ELSE failures END, \
                next_due_at = NULL, updated_at = now() \
         WHERE id = $1 RETURNING ",
        check_columns!()
    ))
    .bind(id)
    .bind(paused)
    .fetch_optional(conn)
    .await?
    .ok_or(Error::NotFound(Missing::Check(id)))?;
    Check::try_from(row)
}

/// The stored HTTP setup of the check, its configuration and opened secrets read in one
/// statement, so a probe never sends the secrets where an older read pointed (SEC-005).
/// `None` when the check is not an HTTP check and holds no secrets.
pub(in crate::checks) async fn load_http_secrets<'e>(
    db: impl PgExecutor<'e>,
    vault: &Vault,
    id: CheckId,
) -> Result<Option<HttpSetup>, Error> {
    let row: Option<(String, Value, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT type, config, secrets_sealed FROM checks.checks WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?;
    let (kind, config, sealed) = row.ok_or(Error::NotFound(Missing::Check(id)))?;
    let config = match config_of(&kind, config) {
        Some(CheckConfig::Http(config)) => config,
        _ if sealed.is_none() => return Ok(None),
        _ => return Err(Error::Unreadable(id)),
    };
    let secrets = sealed
        .map(|bytes| open(vault, id, &config, &bytes))
        .transpose()?;
    Ok(Some(HttpSetup { config, secrets }))
}

/// The stored SNMP setup of the check, read like [`load_http_secrets`]: the secrets open only
/// for the version and user name the stored configuration names (SEC-005). `None` when the
/// check is not an SNMP check and holds no secrets.
pub(in crate::checks) async fn load_snmp_secrets<'e>(
    db: impl PgExecutor<'e>,
    vault: &Vault,
    id: CheckId,
) -> Result<Option<SnmpSetup>, Error> {
    let row: Option<(String, Value, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT type, config, secrets_sealed FROM checks.checks WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?;
    let (kind, config, sealed) = row.ok_or(Error::NotFound(Missing::Check(id)))?;
    let config = match config_of(&kind, config) {
        Some(CheckConfig::Snmp(config)) => config,
        _ if sealed.is_none() => return Ok(None),
        _ => return Err(Error::Unreadable(id)),
    };
    let secrets = sealed
        .map(|bytes| snmp_sealed::open(vault, id, &config, &bytes))
        .transpose()?;
    Ok(Some(SnmpSetup { config, secrets }))
}

/// Moves the check's due time from `expected` to `at`; returns whether it moved. A check
/// whose time changed meanwhile, by a beat or the push route, keeps it (CHK-001).
pub(in crate::checks) async fn set_next_due<'e>(
    db: impl PgExecutor<'e>,
    id: CheckId,
    expected: Option<DateTime<Utc>>,
    at: DateTime<Utc>,
) -> Result<bool, Error> {
    let done = sqlx::query(
        "UPDATE checks.checks SET next_due_at = $3 \
         WHERE id = $1 AND next_due_at IS NOT DISTINCT FROM $2",
    )
    .bind(id)
    .bind(expected)
    .bind(at)
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}
