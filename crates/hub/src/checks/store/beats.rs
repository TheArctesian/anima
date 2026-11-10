//! Heartbeat writes: the raw beat, its three aggregate rows and the check's state, together
//! (CHK-003, DB-008).

use sqlx::{Connection as _, PgConnection};

use super::rows::status_sql;
use crate::{
    checks::{
        Error,
        model::{Beat, Missing, Recorded},
        probe::clean_message,
        uptime::Resolution,
    },
    ids::CheckId,
    status::Status,
};

/// The upsert of one aggregate table: counts add up, and the average response time is kept
/// as a running mean over `response_count` beats.
macro_rules! upsert {
    ($table:literal) => {
        concat!(
            "INSERT INTO checks.",
            $table,
            " AS u (check_id, bucket, up_count, down_count, degraded_count, maintenance_count, \
                    unknown_count, response_count, avg_response_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::real) \
             ON CONFLICT (check_id, bucket) DO UPDATE SET \
                up_count = u.up_count + excluded.up_count, \
                down_count = u.down_count + excluded.down_count, \
                degraded_count = u.degraded_count + excluded.degraded_count, \
                maintenance_count = u.maintenance_count + excluded.maintenance_count, \
                unknown_count = u.unknown_count + excluded.unknown_count, \
                response_count = u.response_count + excluded.response_count, \
                avg_response_ms = CASE WHEN excluded.avg_response_ms IS NULL \
                    THEN u.avg_response_ms \
                    ELSE ((COALESCE(u.avg_response_ms, 0) * u.response_count \
                           + excluded.avg_response_ms) / (u.response_count + 1))::real END"
        )
    };
}

const fn upsert_sql(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Minute => upsert!("uptime_minute"),
        Resolution::Hour => upsert!("uptime_hour"),
        Resolution::Day => upsert!("uptime_day"),
    }
}

/// One count per status, 1 for the beat's status and 0 for the others.
fn counts(status: Status) -> [i32; 5] {
    [
        Status::Up,
        Status::Down,
        Status::Degraded,
        Status::Maintenance,
        Status::Unknown,
    ]
    .map(|each| i32::from(each == status))
}

fn response_ms(beat: &Beat) -> Option<i32> {
    beat.response_time_ms
        .map(|ms| i32::try_from(ms).unwrap_or(i32::MAX))
}

/// Records the beat in the caller's transaction, in a savepoint: all or nothing. The check's
/// state row is written first, so concurrent beats of one check wait for each other. A paused
/// check, or a beat that is not newer than the last one, writes nothing (CHK-005, DB-008).
pub(in crate::checks) async fn record_beat(
    conn: &mut PgConnection,
    check: CheckId,
    beat: &Beat,
) -> Result<Recorded, Error> {
    let mut tx = conn.begin().await?;
    if !write_state(&mut tx, check, beat).await? {
        if exists(&mut tx, check).await? {
            return Ok(Recorded::Skipped);
        }
        return Err(Error::NotFound(Missing::Check(check)));
    }
    sqlx::query("SELECT checks.ensure_day_partition($1)")
        .bind(beat.at)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO checks.heartbeats (check_id, at, status, response_time_ms, message) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(check)
    .bind(beat.at)
    .bind(status_sql(beat.status))
    .bind(response_ms(beat))
    .bind(beat.message.as_deref().map(clean_message))
    .execute(&mut *tx)
    .await?;
    add_to_aggregates(&mut tx, check, beat).await?;
    tx.commit().await?;
    Ok(Recorded::Written)
}

async fn exists(conn: &mut PgConnection, check: CheckId) -> Result<bool, Error> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM checks.checks WHERE id = $1)")
            .bind(check)
            .fetch_one(conn)
            .await?,
    )
}

/// Writes the new state unless the check is paused or already has a later beat; returns
/// whether it did.
async fn write_state(conn: &mut PgConnection, check: CheckId, beat: &Beat) -> Result<bool, Error> {
    let updated = sqlx::query(
        "UPDATE checks.checks SET status = $2, failures = $3, next_due_at = $4, \
                last_beat_at = $5, \
                resolved_ip = COALESCE($6, resolved_ip), \
                cert_not_after = COALESCE($7, cert_not_after), \
                domain_not_after = COALESCE($8, domain_not_after) \
         WHERE id = $1 AND NOT paused AND (last_beat_at IS NULL OR last_beat_at < $5)",
    )
    .bind(check)
    .bind(status_sql(beat.state.status))
    .bind(i32::try_from(beat.state.failures).unwrap_or(i32::MAX))
    .bind(beat.state.next_due_at)
    .bind(beat.at)
    .bind(beat.resolved_ip)
    .bind(beat.cert_not_after)
    .bind(beat.domain_not_after)
    .execute(conn)
    .await?;
    Ok(updated.rows_affected() > 0)
}

/// Adds the beat to its minute, hour and day rows (DB-008).
async fn add_to_aggregates(
    conn: &mut PgConnection,
    check: CheckId,
    beat: &Beat,
) -> Result<(), Error> {
    let [up, down, degraded, maintenance, unknown] = counts(beat.status);
    let response = response_ms(beat);
    #[expect(
        clippy::cast_precision_loss,
        reason = "an average in milliseconds needs no more than f32 precision"
    )]
    let average = response.map(|ms| ms as f32);
    for resolution in Resolution::ALL {
        sqlx::query(upsert_sql(resolution))
            .bind(check)
            .bind(resolution.bucket(beat.at))
            .bind(up)
            .bind(down)
            .bind(degraded)
            .bind(maintenance)
            .bind(unknown)
            .bind(i32::from(response.is_some()))
            .bind(average)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}
