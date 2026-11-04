//! An attempt evaluated on the check's current state and settings and recorded in one
//! transaction (CHK-002, CHK-003, CHK-005).

use chrono::{DateTime, Utc};
use sqlx::{Connection as _, PgConnection};

use super::{
    beats::record_beat,
    rows::{number, status_of},
};
use crate::{
    checks::{
        Error,
        evaluation::{CheckSettings, CheckState, evaluate},
        model::{Attempt, Beat, Missing, Recorded},
        status::maintained,
    },
    ids::CheckId,
};

/// The locked state and settings of one check.
#[derive(sqlx::FromRow)]
struct Locked {
    status: String,
    failures: i32,
    next_due_at: Option<DateTime<Utc>>,
    interval_seconds: i32,
    retries: i32,
    retry_interval_seconds: i32,
    degraded_threshold_ms: i32,
    invert: bool,
}

impl Locked {
    fn state(&self) -> Result<CheckState, sqlx::Error> {
        Ok(CheckState {
            status: status_of(&self.status)?,
            failures: number(self.failures)?,
            next_due_at: self.next_due_at,
        })
    }

    fn settings(&self) -> Result<CheckSettings, sqlx::Error> {
        Ok(CheckSettings {
            interval_seconds: number(self.interval_seconds)?,
            retries: number(self.retries)?,
            retry_interval_seconds: number(self.retry_interval_seconds)?,
            degraded_threshold_ms: number(self.degraded_threshold_ms)?,
            invert: self.invert,
        })
    }
}

/// Locks the check's row, evaluates the attempt on its state and settings and records the
/// beat. Returns the beat when it was written; `None` when `record_beat` skipped it because
/// the check is paused or has a beat at least as new.
pub(in crate::checks) async fn record_attempt(
    conn: &mut PgConnection,
    check: CheckId,
    attempt: &Attempt<'_>,
) -> Result<Option<Beat>, Error> {
    let mut tx = conn.begin().await?;
    let locked: Locked = sqlx::query_as(
        "SELECT status, failures, next_due_at, interval_seconds, retries, \
                retry_interval_seconds, degraded_threshold_ms, invert \
         FROM checks.checks WHERE id = $1 FOR UPDATE",
    )
    .bind(check)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound(Missing::Check(check)))?;
    let (state, settings) = (locked.state()?, locked.settings()?);
    let (state, status) = evaluate(&state, attempt.result, &settings, attempt.at);
    let shown = maintained(status, attempt.in_maintenance);
    let beat = Beat::of(attempt.result.clone(), (state, shown), attempt.at);
    let recorded = record_beat(&mut tx, check, &beat).await?;
    tx.commit().await?;
    Ok((recorded == Recorded::Written).then_some(beat))
}
