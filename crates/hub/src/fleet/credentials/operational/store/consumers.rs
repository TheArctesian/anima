//! The SQL for `credentials.operational_key_consumers`: the grants, and what each daemon holds
//! and fetched (K-E5 Section 2.1).

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::Made;
use crate::ids::{DeviceId, OperationalKeyId};

const DEVICE_FKEY: &str = "operational_key_consumers_device_id_fkey";

/// What [`insert`] did.
pub(in crate::fleet::credentials::operational) enum Granted {
    Made,
    /// The grant existed already: nothing was written.
    Kept,
    /// No device has the id.
    NoDevice,
}

/// Grants the key to `device` for `workload`.
pub(in crate::fleet::credentials::operational) async fn insert(
    conn: &mut PgConnection,
    (key, device, workload): (OperationalKeyId, DeviceId, &str),
    (at, by): Made<'_>,
) -> Result<Granted, sqlx::Error> {
    let done = sqlx::query(
        "INSERT INTO credentials.operational_key_consumers \
         (key_id, device_id, workload, granted_at, granted_by) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT DO NOTHING",
    )
    .bind(key)
    .bind(device)
    .bind(workload)
    .bind(at)
    .bind(by)
    .execute(conn)
    .await;
    match done {
        Ok(done) if done.rows_affected() > 0 => Ok(Granted::Made),
        Ok(_) => Ok(Granted::Kept),
        Err(error)
            if error
                .as_database_error()
                .and_then(|database| database.constraint())
                == Some(DEVICE_FKEY) =>
        {
            Ok(Granted::NoDevice)
        }
        Err(error) => Err(error),
    }
}

/// Ends the grant; whether there was one.
pub(in crate::fleet::credentials::operational) async fn delete(
    conn: &mut PgConnection,
    (key, device, workload): (OperationalKeyId, DeviceId, &str),
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query(
        "DELETE FROM credentials.operational_key_consumers \
         WHERE key_id = $1 AND device_id = $2 AND workload = $3",
    )
    .bind(key)
    .bind(device)
    .bind(workload)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Records that `device` fetched version `number` of the key at `at`, on each of its rows.
pub(in crate::fleet::credentials::operational) async fn mark_fetched(
    conn: &mut PgConnection,
    (key, device): (OperationalKeyId, DeviceId),
    (number, at): (i32, DateTime<Utc>),
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE credentials.operational_key_consumers SET fetched_version = $3, fetched_at = $4 \
         WHERE key_id = $1 AND device_id = $2",
    )
    .bind(key)
    .bind(device)
    .bind(number)
    .bind(at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Records what the daemon of `device` holds at `at`: on each of its rows, the version of that
/// key in `held` (by name), or none when `held` does not name the key.
pub(in crate::fleet::credentials::operational) async fn record_held(
    conn: &mut PgConnection,
    device: DeviceId,
    (names, versions): (&[String], &[i32]),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE credentials.operational_key_consumers c SET held_seen_at = $4, held_version = \
         (SELECT h.version FROM unnest($2::text[], $3::int4[]) AS h(name, version) \
          JOIN credentials.operational_keys k ON k.name = h.name \
          WHERE k.id = c.key_id LIMIT 1) \
         WHERE c.device_id = $1",
    )
    .bind(device)
    .bind(names)
    .bind(versions)
    .bind(at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Each key `device` may get at `now`, by name, with its active version: the keys it is a
/// consumer of that are not removed, not past their expiry, and have an active version.
pub(in crate::fleet::credentials::operational) async fn current_for(
    conn: &mut PgConnection,
    device: DeviceId,
    now: DateTime<Utc>,
) -> Result<Vec<(String, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT DISTINCT k.name, v.number FROM credentials.operational_key_consumers c \
         JOIN credentials.operational_keys k ON k.id = c.key_id \
         JOIN credentials.operational_key_versions v ON v.key_id = k.id AND v.state = 'active' \
         WHERE c.device_id = $1 AND k.removed_at IS NULL \
         AND (k.expires_at IS NULL OR k.expires_at > $2) ORDER BY k.name",
    )
    .bind(device)
    .bind(now)
    .fetch_all(conn)
    .await
}
