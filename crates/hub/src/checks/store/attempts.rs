//! An attempt evaluated on the check's current state and recorded in one transaction
//! (CHK-002, CHK-003, CHK-005).

use sqlx::{Connection as _, PgConnection};

use super::{
    beats::record_beat,
    rows::{number, status_of},
};
use crate::{
    checks::{
        Error,
        evaluation::{CheckState, evaluate},
        model::{Attempt, Beat, Missing, Recorded},
        status::maintained,
    },
    ids::CheckId,
};

/// Locks the check's state row, evaluates the attempt on it and records the beat. Returns the
/// beat when it was written; `None` when the check is paused or has a later beat.
pub(in crate::checks) async fn record_attempt(
    conn: &mut PgConnection,
    check: CheckId,
    attempt: &Attempt<'_>,
) -> Result<Option<Beat>, Error> {
    let mut tx = conn.begin().await?;
    let row: Option<(String, i32, Option<chrono::DateTime<chrono::Utc>>, bool)> = sqlx::query_as(
        "SELECT status, failures, next_due_at, paused FROM checks.checks \
             WHERE id = $1 FOR UPDATE",
    )
    .bind(check)
    .fetch_optional(&mut *tx)
    .await?;
    let (status, failures, next_due_at, paused) =
        row.ok_or(Error::NotFound(Missing::Check(check)))?;
    if paused {
        return Ok(None);
    }
    let current = CheckState {
        status: status_of(&status)?,
        failures: number(failures)?,
        next_due_at,
    };
    let (state, status) = evaluate(&current, attempt.result, attempt.settings, attempt.at);
    let shown = maintained(status, attempt.covered);
    let beat = Beat::of(attempt.result.clone(), (state, shown), attempt.at);
    let recorded = record_beat(&mut tx, check, &beat).await?;
    tx.commit().await?;
    Ok((recorded == Recorded::Written).then_some(beat))
}
