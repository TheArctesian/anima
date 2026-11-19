//! The jobs of the work route (K10, `contracts/work-route.md`, accepted by R-209; PRO-050,
//! PRO-051; task W3): the queue from which the hub gives work to the daemon of a machine.
//!
//! A job is what the contract calls an item of work. The hub never names a command: a job has a
//! type of a fixed list and the object of that type. The rules are pure (`lifecycle`,
//! `placement`); this file runs them in one transaction with their audit entry (K10 Section 10).
//! The HTTP routes are the daemon's edge in `fleet::metrics::ingest`.

mod lifecycle;
mod model;
mod placement;
mod store;
#[cfg(test)]
mod tests;
mod trail;

use chrono::{DateTime, Utc};
use daemon_core::wire::{
    Claim, Lease, Reported, StopOrder, StopReason, Switch, WorkJob, WorkReport, WorkState, WorkType,
};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

pub use model::{Fault, Job, Machine, NewJob, State};

use crate::{
    fleet::events::{self, Bus, ClientAddr, Operator},
    ids::{DeviceId, JobId},
};
use lifecycle::Refusal;

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

async fn ended<T>(tx: Transaction<'_, Postgres>, outcome: Result<T, Error>) -> Result<T, Error> {
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
    asked: NewJob,
    asker: Asker,
    now: DateTime<Utc>,
) -> Result<Job, Error> {
    validate(&asked)?;
    let job = Job {
        id: JobId::from(Uuid::now_v7()),
        asked,
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

/// Answers the claim of `device` at `now` (K10 Section 4): records what the machine said, ends
/// the attempts whose answer was lost, and gives the machine the job that the placement
/// chooses. `claim.device` is already known to be `device`.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn claim(
    queue: Queue<'_>,
    (device, name): (DeviceId, &str),
    claim: &Claim,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Claimed, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = claim_in(&mut tx, queue.bus, (device, name), claim, (client, now)).await;
    ended(tx, outcome).await
}

async fn claim_in(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, name): (DeviceId, &str),
    claim: &Claim,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Claimed, Error> {
    let known = store::machine_locked(conn, device).await?;
    let first_on = claim.switch == Switch::On && known.is_none_or(|old| old.first_on_at.is_none());
    let machine = Machine {
        device,
        switch: claim.switch,
        unclean: claim.unclean,
        off_by_operator: known.is_some_and(|old| old.off_by_operator),
        off_until: known.and_then(|old| old.off_until),
        first_on_at: known
            .and_then(|old| old.first_on_at)
            .or(first_on.then_some(now)),
    };
    store::save_machine(conn, &machine, now).await?;
    if first_on {
        trail::first_claim(conn, bus, (device, name), now).await?;
    }
    if placement::is_off(&machine, now) {
        return Ok(Claimed::Off);
    }
    let mut holds_one = false;
    for held in store::held_by(conn, device).await? {
        let named = claim
            .holds
            .iter()
            .any(|each| JobId::from(each.id) == held.id && each.attempt == held.attempt);
        if named {
            holds_one = true;
        } else {
            // The answer to the claim of this attempt was lost (SF-011).
            let back = lifecycle::answer_lost(&held, now);
            store::save(conn, &back).await?;
            trail::attempt_ended(conn, &back, (device, "answer_lost"), (client, now)).await?;
        }
    }
    if holds_one {
        return Ok(Claimed::Nothing);
    }
    let queued = store::oldest_queued(conn).await?;
    let Some(chosen) = placement::choose(&queued, device, claim) else {
        return Ok(Claimed::Nothing);
    };
    let job = lifecycle::claimed(chosen, device, now);
    store::save(conn, &job).await?;
    trail::attempt_started(conn, &job, device, (client, now)).await?;
    Ok(Claimed::Job(Box::new(wire_job(&job))))
}

/// The job as the machine gets it.
fn wire_job(job: &Job) -> WorkJob {
    WorkJob {
        id: job.id.into(),
        work_type: job.asked.work_type,
        kind: job.asked.kind,
        attempt: job.attempt,
        lease: Lease {
            until: job.lease_ends_at.unwrap_or(job.changed_at),
        },
        limits: job.asked.limits,
        project: job.asked.project.map(Uuid::from),
        repository: job.asked.repository.clone(),
        secrets: job.asked.secrets.clone(),
        parameters: job.asked.parameters.clone(),
    }
}

/// Answers the report of `device` at `now` (K10 Section 7): the new state, the log part, the
/// new lease and a stop when there is one.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn report(
    queue: Queue<'_>,
    device: DeviceId,
    report: &WorkReport,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Reporting, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = report_in(&mut tx, queue.bus, device, report, (client, now)).await;
    ended(tx, outcome).await
}

async fn report_in(
    conn: &mut PgConnection,
    bus: &Bus,
    device: DeviceId,
    report: &WorkReport,
    (client, now): (ClientAddr, DateTime<Utc>),
) -> Result<Reporting, Error> {
    if let Some(machine) = store::machine_locked(conn, device).await?
        && placement::is_off(&machine, now)
    {
        return Ok(Reporting::Off);
    }
    let Some(job) = store::job_locked(conn, JobId::from(report.id)).await? else {
        return Ok(Reporting::NotHolder);
    };
    let mut next = match lifecycle::reported(&job, device, report, now) {
        Ok(next) => next,
        Err(Refusal::NotHolder) => return Ok(Reporting::NotHolder),
        Err(Refusal::Malformed) => return Ok(Reporting::Malformed),
    };
    if let Some(part) = &report.log
        && lifecycle::log_fits(&job, part.from, part.text.len())
    {
        store::append_log(conn, job.id, part.from, &trail::clean_log(&part.text)).await?;
        next.log_end = job.log_end + part.text.len() as u64;
    }
    store::save(conn, &next).await?;
    if report.state == WorkState::Refused {
        fault(conn, bus, (device, Fault::Refused), now).await?;
        trail::attempt_ended(conn, &next, (device, "refused"), (client, now)).await?;
    } else if next.state.is_final() {
        trail::attempt_ended(conn, &next, (device, next.state.as_str()), (client, now)).await?;
    }
    Ok(Reporting::Reported(Reported {
        lease: Lease {
            until: next.lease_ends_at.unwrap_or(now),
        },
        stop: next.stop.map(|reason| StopOrder { reason }),
        log_end: Some(next.log_end),
    }))
}

/// Counts a job that the machine refused or lost; the fifth in 1 hour gives the machine no
/// work for 1 hour, with an event (K10 Section 11, SF-011).
async fn fault(
    conn: &mut PgConnection,
    bus: &Bus,
    (device, fault): (DeviceId, Fault),
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let recent =
        store::add_fault(conn, (device, fault), now, now - placement::FAULT_WINDOW).await?;
    if let Some(until) = placement::off_until(recent, now) {
        store::set_off_until(conn, device, until).await?;
        trail::set_off(conn, bus, device, now).await?;
    }
    Ok(())
}

/// Ends each lease that ran out by `now` and each attempt that holds a job for longer than its
/// limit (K10 Section 7); the upkeep calls it every 30 seconds. Returns how many jobs changed.
///
/// # Errors
/// `Database` or `Events` when a change cannot be written; nothing is changed then.
pub async fn end_lost(queue: Queue<'_>, now: DateTime<Utc>) -> Result<usize, Error> {
    let mut tx = queue.db.begin().await?;
    let outcome = end_lost_in(&mut tx, queue.bus, now).await;
    ended(tx, outcome).await
}

async fn end_lost_in(
    conn: &mut PgConnection,
    bus: &Bus,
    now: DateTime<Utc>,
) -> Result<usize, Error> {
    let client = trail::HUB_ADDRESS;
    let mut changed = 0;
    for job in store::ended_leases(conn, now).await? {
        let Some(device) = job.holder else { continue };
        let next = lifecycle::lease_lost(&job, now);
        store::save(conn, &next).await?;
        fault(conn, bus, (device, Fault::Lost), now).await?;
        trail::attempt_ended(conn, &next, (device, "lease_ended"), (client, now)).await?;
        changed += 1;
    }
    for job in store::over_time(conn, now).await? {
        let Some(device) = job.holder else { continue };
        if !lifecycle::overdue(&job, now) {
            continue;
        }
        let next = lifecycle::expired(&job, now);
        store::save(conn, &next).await?;
        trail::attempt_ended(conn, &next, (device, "expired"), (client, now)).await?;
        changed += 1;
    }
    Ok(changed)
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
        trail::stop_asked(&mut tx, &next, reason, asker, now).await?;
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
