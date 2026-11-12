//! The tries of the publisher (D-25): taking a job for one try and holding it, recording how
//! the try ended, and removing the objects of a job that failed before any feed was sent for it
//! (BLD-070). Each step reads the clock again, so a try that ran for minutes records the time it
//! ended.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use super::{
    run::{Outcome, Reach, Tried, Work},
    transfer,
};
use crate::{
    builds::{BuildJob, BuildQueue, Error, JobState, JobUpdate, Missing, jobs, queue, store},
    events::{self, Bus, NewEvent, Severity, Source},
};

/// How long one try holds a job; a hub that stopped mid-publish is taken over after it.
pub(super) const HOLD: TimeDelta = TimeDelta::minutes(10);

/// How long the publisher waits after the bucket could not be reached.
const RETRY: TimeDelta = TimeDelta::minutes(1);

/// The clock difference allowed between the hub and the bucket: a job is taken only this long
/// after its last upload URL expired (D-28).
const SKEW_MARGIN: TimeDelta = TimeDelta::minutes(2);

/// Tries of one job before it fails.
const MAX_ATTEMPTS: i32 = 3;

/// The reason of a job whose tries all stopped without an end.
const STOPPED: &str = "Publishing stopped 3 times without an end.";

/// The reason of a job whose bucket could not be reached on its last try.
const UNREACHABLE: &str = "The release bucket could not be reached 3 times.";

/// The reason of a job whose tries ran out after a feed write was sent for it, while no try
/// could read what `latest.json` holds (D-25).
const FEED_UNKNOWN: &str = "Feed state unknown, check latest.json.";

/// Takes the next publishable job at `now` for one try and holds it: one whose hold ended and
/// whose upload URLs all expired (D-25, D-28). The work is `None` when its tries are spent, so
/// the caller fails it.
pub(super) async fn take(
    db: &PgPool,
    now: DateTime<Utc>,
) -> Result<Option<(Tried, Option<Work>)>, Error> {
    let mut tx = db.begin().await?;
    let Some((job, attempts)) = store::next_publishable(&mut tx, now, now - SKEW_MARGIN).await?
    else {
        return Ok(None);
    };
    if attempts >= MAX_ATTEMPTS {
        tx.commit().await?;
        let tried = Tried {
            job: job.id,
            attempt: attempts,
        };
        return Ok(Some((tried, None)));
    }
    let attempt = attempts + 1;
    store::hold(&mut tx, job.id, Some(attempt), now + HOLD).await?;
    let files = store::files_of(&mut *tx, job.id).await?;
    let feed_sent = store::feed_sent(&mut *tx, job.id).await?;
    let feed = store::feed_of(&mut *tx, job.id).await?.unwrap_or_default();
    let published = store::published_versions(&mut *tx, job.target).await?;
    tx.commit().await?;
    let work = Work {
        target: job.target,
        version: job.version,
        files,
        feed,
        floor: published.iter().filter_map(|v| v.parse().ok()).max(),
        feed_sent,
    };
    Ok(Some((
        Tried {
            job: job.id,
            attempt,
        },
        Some(work),
    )))
}

/// What the end of a try does to its job.
enum End {
    /// Published, with the feed folders and the files changed after the check.
    Publish(&'static [&'static str], Vec<String>),
    Fail(String),
    /// Wait [`RETRY`], then try again.
    Retry,
    /// Nothing: the try no longer holds the job.
    Leave,
}

/// The end of try `attempt` that had `outcome`: a refusal fails the job, and so do spent tries
/// and an unreachable bucket on the last try (BLD-070); with `feed_sent`, those two say the feed
/// is unknown (D-25).
fn end_of(outcome: Outcome, attempt: i32, feed_sent: bool) -> End {
    let gave_up =
        |reason: &str| End::Fail((if feed_sent { FEED_UNKNOWN } else { reason }).to_owned());
    match outcome {
        Outcome::Published(folders, changed) => End::Publish(folders, changed),
        Outcome::Refused(reason) => End::Fail(reason),
        Outcome::TakenOver => End::Leave,
        Outcome::Spent => gave_up(STOPPED),
        Outcome::Unavailable(fault) => {
            tracing::warn!(
                domain = "builds",
                fault,
                attempt,
                "publishing could not reach the bucket"
            );
            if attempt < MAX_ATTEMPTS {
                End::Retry
            } else {
                gave_up(UNREACHABLE)
            }
        }
    }
}

/// Records how `tried` ended, unless a later try holds the job now; removes the objects of a
/// job that failed before a feed write was sent for it (BLD-070, D-25).
pub(super) async fn finish(
    build_queue: BuildQueue<'_>,
    reach: Reach<'_>,
    tried: Tried,
    outcome: Outcome,
) -> Result<(), Error> {
    let mut tx = build_queue.db.begin().await?;
    let job = store::job_locked(&mut tx, tried.job)
        .await?
        .ok_or(Error::NotFound(Missing::Job(tried.job)))?;
    let current = store::publish_attempt(&mut *tx, tried.job).await?;
    // The attempt alone does not say the job is still the publisher's: a job failed while
    // publishing was off keeps its last attempt.
    let held = job.state == JobState::Uploading && current == Some(tried.attempt);
    let feed_sent = store::feed_sent(&mut *tx, tried.job).await?;
    let end = if held {
        end_of(outcome, tried.attempt, feed_sent)
    } else {
        End::Leave
    };
    let now = reach.clock.now();
    let Some(next) = record(tx, build_queue.bus, (&job, end), now).await? else {
        return Ok(());
    };
    build_queue.updates.publish(JobUpdate {
        job: next.clone(),
        lines: Vec::new(),
    });
    // A feed that was sent may name the files, so they stay (D-25).
    if next.state == JobState::Failed && !feed_sent {
        remove_objects(build_queue.db, reach, &next).await?;
    }
    Ok(())
}

/// Writes `end` for the locked `job` at `now` and ends `tx`; the job as it ended, or `None` when
/// it did not end. `tx` is always committed or rolled back here: a dropped transaction rolls back
/// later, from a task, so its row lock could outlive the try and the sweep's next take would skip
/// the job (D-25).
async fn record(
    mut tx: Transaction<'_, Postgres>,
    bus: &Bus,
    (job, end): (&BuildJob, End),
    now: DateTime<Utc>,
) -> Result<Option<BuildJob>, Error> {
    let next = match end {
        End::Leave => {
            tx.rollback().await?;
            return Ok(None);
        }
        End::Retry => {
            store::hold(&mut tx, job.id, None, now + RETRY).await?;
            tx.commit().await?;
            return Ok(None);
        }
        End::Publish(folders, changed) => {
            let next = queue::published(job, now);
            store::save(&mut tx, &next).await?;
            store::set_written(&mut tx, job.id, folders).await?;
            let event = published_event(job, folders);
            events::record(&mut tx, bus, now, event).await?;
            record_changed(&mut tx, bus, now, (job, &changed)).await?;
            next
        }
        End::Fail(reason) => {
            let next = queue::publish_failed(job, &reason, now);
            store::save(&mut tx, &next).await?;
            jobs::record_failure(&mut tx, bus, &next).await?;
            next
        }
    };
    tx.commit().await?;
    Ok(Some(next))
}

/// Deletes the objects `job`'s upload URLs were given for, so a failed build is never in the
/// bucket for people (BLD-070). A fault is logged and the objects stay.
async fn remove_objects(db: &PgPool, reach: Reach<'_>, job: &BuildJob) -> Result<(), Error> {
    let version = job.version.to_string();
    // A version is built once (D-27), so no published job shares these objects today; the
    // check keeps a published file safe should that ever change.
    if store::is_published(db, job.target, &version).await? {
        return Ok(());
    }
    for file in store::files_of(db, job.id).await? {
        let object = format!("{}/{version}/{}", job.target, file.name);
        let delete = reach.bucket.delete(&object, reach.clock.now());
        if transfer::send(reach.http, delete, Vec::new(), &file.name)
            .await
            .is_err()
        {
            tracing::warn!(
                domain = "builds",
                fault = "bucket delete",
                "publishing could not remove a file of a failed job"
            );
        }
    }
    Ok(())
}

/// The event of `job` published with `folders` written (BLD-068, BLD-082).
fn published_event(job: &BuildJob, folders: &[&str]) -> NewEvent {
    let feed = if folders.is_empty() {
        " without a feed (BLD-068)"
    } else {
        ""
    };
    NewEvent {
        severity: Severity::Info,
        source: Source::Hub,
        device: None,
        message: format!(
            "Anima {} for {} is published{feed}.",
            job.version, job.target
        ),
    }
}

/// Records the warning that `changed`, files of the published `job`, hold other bytes in the
/// bucket than the hub checked (D-28); nothing when none changed.
async fn record_changed(
    tx: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
    (job, changed): (&BuildJob, &[String]),
) -> Result<(), Error> {
    if changed.is_empty() {
        return Ok(());
    }
    let event = NewEvent {
        severity: Severity::Warning,
        source: Source::Hub,
        device: None,
        message: format!(
            "Anima {} for {} is published, but {} changed in the release bucket after the hub \
             checked it. Check that the hub's clock keeps time (NTP).",
            job.version,
            job.target,
            changed.join(" and ")
        ),
    };
    events::record(tx, bus, now, event).await?;
    Ok(())
}
