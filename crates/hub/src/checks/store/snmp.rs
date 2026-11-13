//! The sealed SNMP secrets of a check, opened only for the device host they were entered for,
//! and removed when the device host changed (SEC-005, SEC-010).

use serde_json::Value;
use sqlx::{PgConnection, PgExecutor};

use super::{rows::config_of, sealed::open_snmp};
use crate::{
    checks::{
        Error,
        config::CheckConfig,
        model::{Missing, SnmpSetup},
    },
    credentials::Vault,
    ids::CheckId,
};

/// The type, configuration, sealed secrets and legacy host of a check row.
type SnmpRow = (String, Value, Option<Vec<u8>>, Option<String>);

/// An [`SnmpRow`] with the check's name.
type NamedSnmpRow = (String, Value, Option<Vec<u8>>, Option<String>, String);

/// The stored SNMP setup of the check, read like `load_http_secrets`: the secrets open only
/// for the version and user name the stored configuration names, and for `host`, the device
/// host the probe sends to (SEC-005, SEC-010). `None` when the check is not an SNMP check and
/// holds no secrets.
pub(in crate::checks) async fn load_snmp_secrets<'e>(
    db: impl PgExecutor<'e>,
    vault: &Vault,
    id: CheckId,
    host: &str,
) -> Result<Option<SnmpSetup>, Error> {
    let row: Option<SnmpRow> = sqlx::query_as(
        "SELECT type, config, secrets_sealed, snmp_legacy_host FROM checks.checks WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    let (kind, config, sealed, legacy_host) = row.ok_or(Error::NotFound(Missing::Check(id)))?;
    let config = match config_of(&kind, config) {
        Some(CheckConfig::Snmp(config)) => config,
        _ if sealed.is_none() => return Ok(None),
        _ => return Err(Error::Unreadable(id)),
    };
    let secrets = sealed
        .map(|bytes| open_snmp(vault, id, (&config, host), (&bytes, legacy_host.as_deref())))
        .transpose()?;
    Ok(Some(SnmpSetup { config, secrets }))
}

/// Removes the check's SNMP secrets, in the caller's transaction, when they were entered for
/// another host than `host`; returns the check's name when it removed them. The row is locked
/// and opened again first, so secrets entered meanwhile for `host` are kept.
pub(in crate::checks) async fn forget_snmp_secrets(
    conn: &mut PgConnection,
    vault: &Vault,
    id: CheckId,
    host: &str,
) -> Result<Option<String>, Error> {
    let row: Option<NamedSnmpRow> = sqlx::query_as(
        "SELECT type, config, secrets_sealed, snmp_legacy_host, name FROM checks.checks \
         WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    let (kind, config, sealed, legacy_host, name) =
        row.ok_or(Error::NotFound(Missing::Check(id)))?;
    let (Some(CheckConfig::Snmp(config)), Some(bytes)) = (config_of(&kind, config), sealed) else {
        return Ok(None);
    };
    match open_snmp(vault, id, (&config, host), (&bytes, legacy_host.as_deref())) {
        Err(Error::SecretsForOtherHost) => {}
        _ => return Ok(None),
    }
    sqlx::query(
        "UPDATE checks.checks SET secrets_sealed = NULL, snmp_legacy_host = NULL, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(Some(name))
}
