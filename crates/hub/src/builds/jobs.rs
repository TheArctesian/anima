//! What builders do to jobs, and the end of their leases: claim, report, report the upload
//! (BLD-004 to BLD-010, BLD-048, BLD-063, D-2 to D-5, D-25). Each runs in its own transaction,
//! which it always commits or rolls back, and sends the changed jobs to the live view after it
//! commits.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use super::{
    Error, JobUpdate, Updates, builders, log_lines,
    model::{BuildJob, Builder, FieldError, JobState, LogLine, Missing, Report, UploadedFile},
    queue::{self, Refusal},
    store, upload,
};
use crate::{
    events::{self, Bus, NewEvent, Severity, Source},
    ids::BuildJobId,
};

/// The build queue's database and where its changes are announced: failed jobs to the event
/// log's bus, every change to the Builds view (BLD-081, BLD-082).
#[derive(Clone, Copy)]
pub struct BuildQueue<'a> {
    pub db: &'a PgPool,
    pub bus: &'a Bus,
    pub updates: &'a Updates,
}

impl BuildQueue<'_> {
    fn announce(self, jobs: Vec<BuildJob>) {
        for job in jobs {
            self.updates.publish(JobUpdate {
                job,
                lines: Vec::new(),
            });
        }
    }
}

/// A builder's report that the job's files are uploaded (BLD-063, D-5). No `Debug`: its feed is
/// builder text (LOG-007).
#[derive(Clone)]
pub struct Upload {
    pub job: BuildJobId,
    pub files: Vec<UploadedFile>,
    pub feed: String,
}

impl From<Refusal> for Error {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::NotHolder => Self::Forbidden("This builder does not hold the job's lease."),
            Refusal::LeaseEnded => {
                Self::Conflict("The lease ended; the job goes back to the queue.".to_owned())
            }
            Refusal::Change { from, to } => Self::Conflict(format!(
                "A {} build job cannot become {}.",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Commits `tx` when `outcome` is a value and rolls it back when it is an error, so no row lock
/// outlives the call: a dropped transaction rolls back only later, from a task, and until then
/// a claim or the publisher's take (`SKIP LOCKED`) passes its rows by. A rollback that fails is
/// returned in place of `outcome`'s error.
pub(super) async fn ended<T>(
    tx: Transaction<'_, Postgres>,
    outcome: Result<T, Error>,
) -> Result<T, Error> {
    match outcome {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}

/// Writes the event of a job that failed (BLD-082). Its reason is redacted by `record`.
pub(super) async fn record_failure(
    conn: &mut PgConnection,
    bus: &Bus,
    job: &BuildJob,
) -> Result<(), Error> {
    let reason = job.failure_reason.as_deref().unwrap_or_default();
    let event = NewEvent {
        severity: Severity::Error,
        source: Source::Hub,
        device: None,
        message: format!(
            "The build of {} for {} failed: {reason}.",
            job.version, job.target
        ),
    };
    events::record(conn, bus, job.times.changed, event).await?;
    Ok(())
}

/// Returns each job whose lease ended to the queue, or fails it on the third time (BLD-006),
/// in the caller's transaction. Returns the changed jobs.
async fn expire_in(
    conn: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<Vec<BuildJob>, Error> {
    let mut changed = Vec::new();
    for job in store::ended_leases(conn, now).await? {
        let next = queue::expire(&job, now);
        store::save(conn, &next).await?;
        if next.state == JobState::Failed {
            record_failure(conn, bus, &next).await?;
        }
        changed.push(next);
    }
    Ok(changed)
}

/// Ends every lease that ran out by `now` (D-2); the upkeep job calls it every 30 seconds.
/// Returns how many jobs changed.
///
/// # Errors
///
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn expire_leases(
    build_queue: BuildQueue<'_>,
    now: DateTime<Utc>,
) -> Result<usize, Error> {
    let mut tx = build_queue.db.begin().await?;
    let outcome = expire_in(&mut tx, build_queue.bus, now).await;
    let changed = ended(tx, outcome).await?;
    let count = changed.len();
    build_queue.announce(changed);
    Ok(count)
}

/// Gives `builder` the oldest queued job it can build, with a 90-second lease, after ending the
/// leases that ran out (BLD-003 to BLD-005, D-2, D-3). `None` when no job waits, or when the
/// builder still holds a lease: it runs one job at a time (BLD-029).
///
/// # Errors
///
/// `Forbidden` when the builder is no longer enrolled; `Database` or `Events` when the claim
/// cannot be written. Nothing is changed then.
pub async fn claim(
    build_queue: BuildQueue<'_>,
    builder: &Builder,
    now: DateTime<Utc>,
) -> Result<Option<BuildJob>, Error> {
    let mut tx = build_queue.db.begin().await?;
    let outcome = claim_in(&mut tx, build_queue.bus, builder, now).await;
    let (mut changed, claimed) = ended(tx, outcome).await?;
    changed.extend(claimed.clone());
    build_queue.announce(changed);
    Ok(claimed)
}

/// [`claim`] in the caller's transaction: the jobs whose leases ended, and the claimed job.
async fn claim_in(
    conn: &mut PgConnection,
    bus: &Bus,
    builder: &Builder,
    now: DateTime<Utc>,
) -> Result<(Vec<BuildJob>, Option<BuildJob>), Error> {
    // First: a second claim of the same builder waits here until this one commits, so it sees
    // the lease this one takes (BLD-029).
    if !store::lock_builder(conn, builder.id).await? {
        return Err(Error::Forbidden(builders::NOT_ENROLLED));
    }
    let changed = expire_in(conn, bus, now).await?;
    if store::holds_lease(&mut *conn, builder.id, now).await? {
        return Ok((changed, None));
    }
    let targets = queue::buildable(builder.platform);
    let Some(job) = store::oldest_queued(conn, &targets).await? else {
        return Ok((changed, None));
    };
    let claimed = queue::claim(&job, builder, now);
    store::save(conn, &claimed).await?;
    // A key block an earlier attempt left open must not hide this attempt's log.
    store::set_open_key_block(conn, claimed.id, None).await?;
    Ok((changed, Some(claimed)))
}

/// Adds the lines to the job's log after its last line, then trims the log to 2 MB (BLD-048).
/// Returns them as stored.
async fn append(
    conn: &mut PgConnection,
    job: &mut BuildJob,
    lines: Vec<String>,
) -> Result<Vec<LogLine>, Error> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let last = job.log_sequence;
    store::append_lines(conn, job.id, last, &lines).await?;
    let (max, overhead) = (log_lines::MAX_JOB_LOG_BYTES, log_lines::LINE_OVERHEAD_BYTES);
    store::trim_log(conn, job.id, max, overhead).await?;
    let stored: Vec<LogLine> = (last + 1..)
        .zip(lines)
        .map(|(sequence, text)| LogLine { sequence, text })
        .collect();
    job.log_sequence = stored.last().map_or(last, |line| line.sequence);
    Ok(stored)
}

/// Records the holder's report: the state change it names, the lease extended to 90 seconds
/// from `now`, and its log lines redacted (BLD-005, BLD-010, BLD-049).
///
/// # Errors
///
/// `NotFound` for no such job; `Forbidden` when `builder` does not hold its lease; `Conflict`
/// for a lease that ended, a change the diagram does not permit or a job the hub publishes;
/// `Validation` (`logLines`) for too many lines.
pub async fn report(
    build_queue: BuildQueue<'_>,
    builder: &Builder,
    report: Report,
    now: DateTime<Utc>,
) -> Result<BuildJob, Error> {
    let mut tx = build_queue.db.begin().await?;
    let outcome = report_in(&mut tx, build_queue.bus, (builder, &report), now).await;
    let (next, stored) = ended(tx, outcome).await?;
    build_queue.updates.publish(JobUpdate {
        job: next.clone(),
        lines: stored,
    });
    Ok(next)
}

/// [`report`] in the caller's transaction: the job as reported, and its new log lines.
async fn report_in(
    conn: &mut PgConnection,
    bus: &Bus,
    (builder, report): (&Builder, &Report),
    now: DateTime<Utc>,
) -> Result<(BuildJob, Vec<LogLine>), Error> {
    let job = store::job_locked(conn, report.job)
        .await?
        .ok_or(Error::NotFound(Missing::Job(report.job)))?;
    if store::is_publishing(&mut *conn, job.id).await? {
        return Err(Error::Conflict(PUBLISHING.to_owned()));
    }
    let mut next = queue::report(&job, builder.id, report, now)?;
    let was_open = store::open_key_block(conn, job.id).await?;
    let (lines, open) =
        log_lines::clean_all(&report.lines, was_open).map_err(|e| Error::Validation(vec![e]))?;
    if open != was_open {
        store::set_open_key_block(conn, job.id, open).await?;
    }
    let stored = append(conn, &mut next, lines).await?;
    store::save(conn, &next).await?;
    if next.state == JobState::Failed {
        record_failure(conn, bus, &next).await?;
    }
    Ok((next, stored))
}

/// The refusal of a builder call on a job whose files are in: the hub owns it now (D-25).
const PUBLISHING: &str = "The hub is publishing this job.";

/// The `uploading` job `id`, locked, as its holder `builder` extends the lease at `now`; not yet
/// saved. Refused once the hub publishes it.
pub(super) async fn extend_uploading(
    conn: &mut PgConnection,
    builder: &Builder,
    id: BuildJobId,
    now: DateTime<Utc>,
) -> Result<BuildJob, Error> {
    let job = store::job_locked(conn, id)
        .await?
        .ok_or(Error::NotFound(Missing::Job(id)))?;
    if store::is_publishing(&mut *conn, id).await? {
        return Err(Error::Conflict(PUBLISHING.to_owned()));
    }
    let extend = Report {
        job: id,
        state: None,
        reason: None,
        lines: Vec::new(),
    };
    let next = queue::report(&job, builder.id, &extend, now)?;
    if next.state != JobState::Uploading {
        return Err(Error::Conflict(
            "Upload the files while the job is uploading.".to_owned(),
        ));
    }
    Ok(next)
}

/// Whether `reported` names exactly the files of the grant, each with its size and SHA-256.
fn same_files(mut reported: Vec<UploadedFile>, granted: &[UploadedFile]) -> bool {
    reported.sort_by(|a, b| a.name.cmp(&b.name));
    reported == granted
}

/// Records the holder's proposed feed for an `uploading` job whose files are the grant of its
/// upload URLs, ends the builder's lease and gives the job to the publisher (BLD-063, D-25,
/// D-26). The job stays `uploading` until the publisher ends it.
///
/// # Errors
///
/// As [`report`], and `Conflict` when the job is not `uploading` or the hub publishes it;
/// `Validation` (`files`, `feed`) for a bad feed or files other than the grant.
pub async fn report_uploaded(
    build_queue: BuildQueue<'_>,
    builder: &Builder,
    uploaded: Upload,
    now: DateTime<Utc>,
) -> Result<BuildJob, Error> {
    let files = upload::files(&uploaded.files).map_err(|e| Error::Validation(vec![e]))?;
    let feed = upload::feed(&uploaded.feed).map_err(|e| Error::Validation(vec![e]))?;
    let mut tx = build_queue.db.begin().await?;
    let outcome = hand_over(&mut tx, builder, (uploaded.job, files, &feed), now).await;
    let next = ended(tx, outcome).await?;
    build_queue.updates.publish(JobUpdate {
        job: next.clone(),
        lines: Vec::new(),
    });
    Ok(next)
}

/// [`report_uploaded`] in the caller's transaction, with the files and feed already read: the
/// job as handed to the publisher.
async fn hand_over(
    conn: &mut PgConnection,
    builder: &Builder,
    (id, files, feed): (BuildJobId, Vec<UploadedFile>, &str),
    now: DateTime<Utc>,
) -> Result<BuildJob, Error> {
    let mut next = extend_uploading(conn, builder, id, now).await?;
    let granted = store::files_of(&mut *conn, next.id).await?;
    if !same_files(files, &granted) {
        let message = "Report the files the upload URLs were given for.";
        return Err(Error::Validation(vec![FieldError::new("files", message)]));
    }
    store::set_feed(conn, next.id, feed, now).await?;
    store::save(conn, &next).await?;
    store::hand_to_publisher(conn, next.id, now).await?;
    next.lease_ends_at = None;
    Ok(next)
}
