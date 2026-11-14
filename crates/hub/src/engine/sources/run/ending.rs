//! The end of a run: the account's claim, counts and next sync, the run row, and one warning
//! when the health turns `STALLED` or `NEEDS_RECONNECT` (K-E1 Section 8, steps 4 and 5).

use chrono::{DateTime, TimeDelta, Utc};

use super::{Claim, Failure, Stop};
use crate::{
    engine::sources::{
        Error, Scheduling,
        health::{HealthFacts, health, worsened},
        model::{Account, Health, Outcome},
        schedule,
        store::{self, Ending, runs},
    },
    fleet::events::{self, ActorKind, NewEvent, Severity, Source},
};

/// Health facts of a running account: its consumer is on, or it would not run.
fn facts(account: &Account, interval: TimeDelta) -> HealthFacts {
    HealthFacts {
        paused: account.paused_at.is_some(),
        consumer_on: true,
        last_error_kind: account.error_kind(),
        last_started_at: account.last_started_at,
        last_sync_at: account.last_sync_at,
        failure_count: account.failures(),
        interval,
    }
}

/// Ends the run of `claim` as `stop` says. A run that lost its claim writes nothing, and so
/// does one whose claim was taken while it ended.
///
/// # Errors
/// `Database` when the end cannot be written.
pub(super) async fn finish(
    scheduling: &Scheduling,
    claim: &Claim,
    stop: Stop,
) -> Result<Option<Outcome>, Error> {
    let (outcome, failure, counted) = match stop {
        Stop::LostClaim => return Ok(None),
        Stop::Done => (Outcome::Ok, None, true),
        Stop::Stopped => (Outcome::Stopped, None, false),
        Stop::Failed(failure) => (Outcome::Failed, Some(failure), true),
    };
    let now = scheduling.clock.now();
    let interval = schedule::interval(claim.declaration.map_or(0, |d| d.poll_interval_seconds));
    let ending = ending(claim, (failure, counted), (now, interval));
    let mut tx = scheduling.pool.begin().await?;
    let Some(ended) = store::end(&mut tx, claim.account.id, &ending).await? else {
        return Ok(None);
    };
    let words = failure.map(|f| f.words);
    runs::finish(
        &mut tx,
        (claim.run, claim.account.id),
        (now, outcome),
        words,
    )
    .await?;
    let before = health(&facts(&claim.account, interval), now);
    let after = health(&facts(&ended, interval), now);
    if worsened(before, after) {
        events::record(&mut tx, &scheduling.bus, now, warning(&ended, after))
            .await
            .map_err(|events::Error::Database(e)| Error::Database(e))?;
    }
    tx.commit().await?;
    scheduling.feed.publish(claim.account.id);
    if let Some(failure) = failure {
        // The kind is a fixed word; the source key is a declared constant (LOG-007).
        tracing::warn!(source = %ended.source, kind = failure.kind.as_str(), "source sync failed");
    }
    Ok(Some(outcome))
}

/// What the account is written as: the claim ends; a success clears the count, a failure adds
/// one, and a stopped run keeps it. The next sync follows `schedule::next`.
fn ending(
    claim: &Claim,
    (failure, counted): (Option<Failure>, bool),
    (now, interval): (DateTime<Utc>, TimeDelta),
) -> Ending {
    let failures = if failure.is_some() {
        claim.account.failures() + 1
    } else {
        0
    };
    Ending {
        generation: claim.generation,
        now,
        failure: failure.map(|f| (f.kind, f.words)),
        counted,
        next_sync_at: schedule::next(now, interval, failures, failure.and_then(|f| f.until)),
    }
}

/// The warning: the account's label and its new health, never an error reply.
fn warning(account: &Account, health: Health) -> NewEvent {
    let state = match health {
        Health::NeedsReconnect => "needs to be connected again",
        _ => "has stalled: its syncs keep failing",
    };
    NewEvent {
        severity: Severity::Warning,
        source: Source::Hub,
        device: None,
        message: format!("Source account {} {state}.", account.label),
        actor: ActorKind::Hub,
    }
}
