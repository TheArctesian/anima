//! The one row of `checks.defaults`: what a new check starts with (D-26, DB-011).

use sqlx::{Connection as _, PgConnection, PgExecutor};

use super::{rows::write_error, writes::small};
use crate::checks::{CheckDefaults, Error};

type Row = (i32, i32, i32, i32);

fn decoded<T: TryFrom<i32>>(value: i32) -> Result<T, sqlx::Error> {
    T::try_from(value).map_err(|_| sqlx::Error::Decode("a stored default is out of range".into()))
}

fn defaults_of((interval, retries, retry_interval, degraded): Row) -> Result<CheckDefaults, Error> {
    Ok(CheckDefaults {
        interval_seconds: decoded(interval)?,
        retries: decoded(retries)?,
        retry_interval_seconds: decoded(retry_interval)?,
        degraded_threshold_ms: decoded(degraded)?,
    })
}

pub(in crate::checks) async fn read<'e>(db: impl PgExecutor<'e>) -> Result<CheckDefaults, Error> {
    let row = sqlx::query_as::<_, Row>(
        "SELECT interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms \
         FROM checks.defaults",
    )
    .fetch_one(db)
    .await?;
    defaults_of(row)
}

/// Replaces the stored defaults in the caller's transaction, in a savepoint. A value outside
/// the bounds of `checks.checks` is refused by name of the field.
pub(in crate::checks) async fn write(
    conn: &mut PgConnection,
    new: CheckDefaults,
) -> Result<CheckDefaults, Error> {
    let mut tx = conn.begin().await?;
    let row = sqlx::query_as::<_, Row>(
        "UPDATE checks.defaults SET interval_seconds = $1, retries = $2, \
                retry_interval_seconds = $3, degraded_threshold_ms = $4 \
         RETURNING interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms",
    )
    .bind(small(new.interval_seconds))
    .bind(i32::from(new.retries))
    .bind(small(new.retry_interval_seconds))
    .bind(small(new.degraded_threshold_ms))
    .fetch_one(&mut *tx)
    .await
    .map_err(write_error)?;
    tx.commit().await?;
    defaults_of(row)
}
