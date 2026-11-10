//! An attempt evaluated on the check's current state and settings and recorded in one
//! transaction, with the notifications its beat calls for (CHK-002, CHK-003, CHK-005, D-16).

use chrono::{DateTime, Utc};
use sqlx::{Connection as _, PgConnection};

use super::{
    beats::record_beat,
    rows::{number, status_of},
};
use crate::{
    checks::{
        Error,
        config::CheckType,
        evaluation::{CheckSettings, CheckState, evaluate},
        model::{Attempt, Beat, Missing, Recorded},
        probe::ProbeResult,
        status::maintained,
    },
    ids::CheckId,
    notifications::{self, Observation},
    status::Status,
};

/// The locked state and settings of one check.
#[derive(sqlx::FromRow)]
struct Locked {
    name: String,
    #[sqlx(rename = "type")]
    kind: String,
    status: String,
    failures: i32,
    next_due_at: Option<DateTime<Utc>>,
    interval_seconds: i32,
    retries: i32,
    retry_interval_seconds: i32,
    degraded_threshold_ms: i32,
    invert: bool,
    /// The domain the stored configuration names, if any.
    domain: Option<String>,
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

/// Locks the check's row, evaluates the attempt on its state and settings, records the beat
/// and queues the notifications a written beat calls for. Returns the beat when it was
/// written; `None` when `record_beat` skipped it because the check is paused or has a beat at
/// least as new.
pub(in crate::checks) async fn record_attempt(
    conn: &mut PgConnection,
    check: CheckId,
    attempt: &Attempt<'_>,
) -> Result<Option<Beat>, Error> {
    let mut tx = conn.begin().await?;
    let locked: Locked = sqlx::query_as(
        "SELECT name, type, status, failures, next_due_at, interval_seconds, retries, \
                retry_interval_seconds, degraded_threshold_ms, invert, \
                config ->> 'domain' AS domain \
         FROM checks.checks WHERE id = $1 FOR UPDATE",
    )
    .bind(check)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(Error::NotFound(Missing::Check(check)))?;
    let kind = CheckType::from_sql(&locked.kind).ok_or(Error::Unreadable(check))?;
    let (state, settings) = (locked.state()?, locked.settings()?);
    let previous = state.status;
    let (state, status) = evaluate(&state, attempt.result, &settings, attempt.at);
    let shown = maintained(status, attempt.in_maintenance);
    let beat = Beat::of(
        dated_for(attempt, locked.domain.as_deref()),
        (state, shown),
        attempt.at,
    );
    let recorded = record_beat(&mut tx, check, &beat).await?;
    if recorded == Recorded::Written {
        let seen = observation((check, kind), &locked, attempt, (previous, &beat));
        notify(&mut tx, &seen).await?;
    }
    tx.commit().await?;
    Ok((recorded == Recorded::Written).then_some(beat))
}

/// What notifications needs to know about the written beat (D-16). Maintenance is passed as
/// the attempt saw it, so no Down is sent under a window (Section 6.5). `previous` is the
/// check's stored status before this beat as evaluated, without the maintenance a beat
/// shows. The site is the device's site id, as checks reads no site names.
fn observation(
    (check, kind): (CheckId, CheckType),
    locked: &Locked,
    attempt: &Attempt<'_>,
    (previous, beat): (Status, &Beat),
) -> Observation {
    Observation {
        check,
        check_name: locked.name.clone(),
        check_type: kind.as_sql(),
        device: (attempt.device.id, attempt.device.name.clone()),
        site: attempt.device.site.to_string(),
        previous,
        status: beat.status,
        in_maintenance: attempt.in_maintenance,
        at: beat.at,
        message: beat.message.clone(),
        certificate_not_after: beat.cert_not_after,
        domain_not_after: beat.domain_not_after,
    }
}

/// Queues the notifications in a savepoint of the beat's transaction. A notification fault
/// (an unreadable row, a failed statement) rolls back only the savepoint and is logged with
/// the check's id and a fixed word, so the beat still commits (R-104, LOG-007).
///
/// # Errors
/// `Database` when the savepoint cannot be opened, released or rolled back: the connection is
/// then broken, and the beat could not commit either.
async fn notify(conn: &mut PgConnection, seen: &Observation) -> Result<(), Error> {
    let mut savepoint = conn.begin().await?;
    match notifications::observe(&mut savepoint, seen).await {
        Ok(()) => savepoint.commit().await?,
        Err(fault) => {
            savepoint.rollback().await?;
            tracing::warn!(
                domain = "notifications",
                check_id = %seen.check,
                fault = fault.fault(),
                "the notifications of a heartbeat could not be queued; the heartbeat is kept"
            );
        }
    }
    Ok(())
}

/// The attempt's result, without its expiry date when the check names another domain than
/// the one probed: a probe of the old domain that ends after a change keeps no date (D-29).
fn dated_for(attempt: &Attempt<'_>, stored: Option<&str>) -> ProbeResult {
    let mut result = attempt.result.clone();
    if attempt.domain != stored {
        result.domain_not_after = None;
    }
    result
}
