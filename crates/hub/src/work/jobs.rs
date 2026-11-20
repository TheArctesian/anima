//! The jobs of the work route (K10, `contracts/work-route.md`, accepted by R-209; PRO-050,
//! PRO-051; task W3): the queue from which the hub gives work to the daemon of a machine.
//!
//! A job is what the contract calls an item of work. The hub never names a command: a job has a
//! type of a fixed list and the object of that type. The rules are pure (`lifecycle`,
//! `placement`); this file runs them in one transaction with their audit entry (K10 Section 10).
//! The HTTP routes are the daemon's edge in `fleet::metrics::ingest`.

mod claiming;
mod lifecycle;
mod model;
mod placement;
mod reporting;
mod store;
#[cfg(test)]
mod tests;
mod trail;

use chrono::{DateTime, Utc};
use daemon_core::wire::{Reported, StopReason, WorkJob, WorkType};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub use claiming::claim;
pub use model::{Fault, Job, Machine, NewJob, State};
pub use reporting::{end_lost, report};

use crate::{
    fleet::events::{self, Bus, ClientAddr, Operator},
    ids::JobId,
};

/// How often the upkeep ends the leases that ran out and the attempts that are too long.
pub const LEASE_SWEEP: std::time::Duration = std::time::Duration::from_secs(30);

/// Why an operation on the jobs failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The job that a caller wants in the queue breaks a rule of K10; the text names the rule.
    #[error("the job is not valid: {0}")]
    Invalid(&'static str),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored row is not in a form this version reads.
    #[error("a stored job row cannot be read")]
    Unreadable,
    #[error("the event log cannot be written")]
    Events(#[from] events::Error),
}

/// What the jobs need to work: the database and the bus of the events.
#[derive(Clone, Copy)]
pub struct Queue<'a> {
    pub db: &'a PgPool,
    pub bus: &'a Bus,
}

/// Who asks, for the audit entry: the operator (or the hub) and the address of the request.
pub type Asker = (Operator, ClientAddr);

/// The answer to a claim.
#[derive(Debug, PartialEq)]
pub enum Claimed {
    /// No job for this machine now (`204`).
    Nothing,
    /// The hub gives this machine no work (`403 WORK_OFF`).
    Off,
    Job(Box<WorkJob>),
}

/// The answer to a report.
#[derive(Debug, PartialEq, Eq)]
pub enum Reporting {
    Reported(Reported),
    /// `409 NOT_HOLDER`.
    NotHolder,
    /// `400 MALFORMED`.
    Malformed,
    /// `403 WORK_OFF`.
    Off,
}

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

/// The purposes that a job of each type can name (K10 Sections 6 and 8).
fn secrets_permitted(job: &NewJob) -> bool {
    let permitted: &[&str] = match job.work_type {
        WorkType::Runner => &["jit_config"],
        WorkType::Build => &["repository_token", "project_secret"],
        WorkType::Unknown => &[],
    };
    job.secrets
        .iter()
        .all(|purpose| permitted.contains(&purpose.as_str()))
}

fn validate(job: &NewJob) -> Result<(), Error> {
    use daemon_core::wire::WorkKind;
    if job.work_type == WorkType::Unknown || job.kind == WorkKind::Unknown {
        return Err(Error::Invalid(
            "the type and the kind are of the fixed lists",
        ));
    }
    if !job.parameters.is_object() {
        return Err(Error::Invalid("the parameters are an object"));
    }
    if !secrets_permitted(job) {
        return Err(Error::Invalid(
            "a secret purpose is not one of the type of the job",
        ));
    }
    let limits = job.limits;
    let all = [
        limits.seconds,
        limits.idle_seconds,
        limits.memory_bytes,
        limits.disk_bytes,
        limits.orphan_seconds,
    ];
    if all
        .iter()
        .any(|&limit| limit == 0 || i64::try_from(limit).is_err())
    {
        return Err(Error::Invalid("each limit is above 0"));
    }
    Ok(())
}

/// Puts a job in the queue at `now`, with its audit entry (K10 Section 10). The callers are the
/// functions of the hub that make work: the runners of a project and the builds.
///
/// # Errors
/// [`Error::Invalid`] for a job that breaks a rule of K10; `Database` or `Events` when it
/// cannot be written.
pub async fn enqueue(
    queue: Queue<'_>,
    wanted: NewJob,
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<Job, Error> {
    validate(&wanted)?;
    let job = Job {
        id: JobId::from(Uuid::now_v7()),
        asked: wanted,
        state: State::Queued,
        attempt: 0,
        lost: 0,
        holder: None,
        last_device: None,
        refused_by: Vec::new(),
        lease_ends_at: None,
        claimed_at: None,
        stop: None,
        reason: None,
        result: None,
        log_end: 0,
        created_at: now,
        changed_at: now,
        ended_at: None,
    };
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        store::insert(&mut tx, &job).await?;
        trail::queued(&mut tx, &job, asker, now).await
    }
    .await;
    ended(tx, outcome).await.map(|()| job)
}

/// Asks for the stop of a job at `now` (K10 Section 7). A queued job stops at once; the holder
/// of a held job gets the stop in the answer to its next report. `None` for a job that is not
/// there or has ended.
///
/// # Errors
/// `Database` or `Events` when the change cannot be written; nothing is changed then.
pub async fn stop(
    queue: Queue<'_>,
    (id, reason): (JobId, StopReason),
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<Option<Job>, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        let Some(job) = store::job_locked(&mut tx, id).await? else {
            return Ok(None);
        };
        let Some(next) = lifecycle::stop_asked(&job, reason, now) else {
            return Ok(None);
        };
        store::save(&mut tx, &next).await?;
        trail::stop_asked(&mut tx, &next, reason, (asker, now)).await?;
        Ok(Some(next))
    }
    .await;
    ended(tx, outcome).await
}

/// The job and the log that the hub holds of it; `None` when there is no such job.
///
/// # Errors
/// `Database` when it cannot be read.
pub async fn job_with_log(queue: Queue<'_>, id: JobId) -> Result<Option<(Job, String)>, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = async {
        let Some(job) = store::job_locked(&mut tx, id).await? else {
            return Ok(None);
        };
        let log = store::log_of(&mut tx, id).await?;
        Ok(Some((job, log)))
    }
    .await;
    ended(tx, outcome).await
}
