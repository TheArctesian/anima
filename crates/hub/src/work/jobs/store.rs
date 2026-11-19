//! All SQL on schema `jobs`: I/O only (CQ-032). Ids and time come in as parameters. Writes take
//! the caller's connection, so a change commits with its audit entry or event.

use chrono::{DateTime, Utc};
use daemon_core::wire::{
    Limits, StopReason, Switch, WorkArchitecture, WorkKind, WorkOs, WorkRepository, WorkType,
};
use sqlx::PgConnection;

use super::{
    Error,
    model::{Fault, Job, Machine, NewJob, State},
};
use crate::ids::{DeviceId, JobId, ProjectId};

/// The most bytes of the log of one job that the hub keeps (K10 Section 7).
const MAX_LOG_BYTES: i64 = 2 * 1024 * 1024;

/// How many queued jobs one claim looks at, oldest first.
const CLAIM_LOOKS_AT: i64 = 50;

const SELECT_JOB: &str = "SELECT id, work_type, kind, os, architecture, project_id, forge, owner, \
     name, parameters, secrets, limit_seconds, limit_idle_seconds, limit_memory_bytes, \
     limit_disk_bytes, limit_orphan_seconds, state, attempt, lost, device_id, last_device_id, \
     refused_by, lease_ends_at, claimed_at, stop_reason, reason, result, log_end, created_at, \
     state_changed_at, ended_at FROM jobs.jobs ";

#[derive(sqlx::FromRow)]
struct JobRow {
    id: JobId,
    work_type: String,
    kind: String,
    os: Option<String>,
    architecture: Option<String>,
    project_id: Option<ProjectId>,
    forge: String,
    owner: String,
    name: String,
    parameters: serde_json::Value,
    secrets: Vec<String>,
    limit_seconds: i64,
    limit_idle_seconds: i64,
    limit_memory_bytes: i64,
    limit_disk_bytes: i64,
    limit_orphan_seconds: i64,
    state: String,
    attempt: i32,
    lost: i32,
    device_id: Option<DeviceId>,
    last_device_id: Option<DeviceId>,
    refused_by: Vec<DeviceId>,
    lease_ends_at: Option<DateTime<Utc>>,
    claimed_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    reason: Option<String>,
    result: Option<serde_json::Value>,
    log_end: i64,
    created_at: DateTime<Utc>,
    state_changed_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
}

/// A word of the contract as the database holds it: the JSON string of its type.
fn word<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => Ok(text),
        _ => Err(Error::Unreadable),
    }
}

fn read_word<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Error> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|_| Error::Unreadable)
}

fn count(value: i64) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::Unreadable)
}

fn stored(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::Unreadable)
}

impl JobRow {
    fn asked(&self) -> Result<NewJob, Error> {
        Ok(NewJob {
            work_type: read_word::<WorkType>(&self.work_type)?,
            kind: read_word::<WorkKind>(&self.kind)?,
            os: self.os.as_deref().map(read_word::<WorkOs>).transpose()?,
            architecture: self
                .architecture
                .as_deref()
                .map(read_word::<WorkArchitecture>)
                .transpose()?,
            project: self.project_id,
            repository: WorkRepository {
                forge: self.forge.clone(),
                owner: self.owner.clone(),
                name: self.name.clone(),
            },
            parameters: self.parameters.clone(),
            secrets: self.secrets.clone(),
            limits: Limits {
                seconds: count(self.limit_seconds)?,
                idle_seconds: count(self.limit_idle_seconds)?,
                memory_bytes: count(self.limit_memory_bytes)?,
                disk_bytes: count(self.limit_disk_bytes)?,
                orphan_seconds: count(self.limit_orphan_seconds)?,
            },
        })
    }
}

impl TryFrom<JobRow> for Job {
    type Error = Error;

    fn try_from(row: JobRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            asked: row.asked()?,
            state: State::from_stored(&row.state).ok_or(Error::Unreadable)?,
            attempt: u32::try_from(row.attempt).map_err(|_| Error::Unreadable)?,
            lost: u8::try_from(row.lost).map_err(|_| Error::Unreadable)?,
            holder: row.device_id,
            last_device: row.last_device_id,
            refused_by: row.refused_by,
            lease_ends_at: row.lease_ends_at,
            claimed_at: row.claimed_at,
            stop: row
                .stop_reason
                .as_deref()
                .map(read_word::<StopReason>)
                .transpose()?,
            reason: row.reason,
            result: row
                .result
                .map(serde_json::from_value)
                .transpose()
                .map_err(|_| Error::Unreadable)?,
            log_end: count(row.log_end)?,
            created_at: row.created_at,
            changed_at: row.state_changed_at,
            ended_at: row.ended_at,
        })
    }
}

async fn jobs_where(
    conn: &mut PgConnection,
    condition: &str,
    at: Option<DateTime<Utc>>,
    device: Option<DeviceId>,
) -> Result<Vec<Job>, Error> {
    let sql = format!("{SELECT_JOB}{condition}");
    let rows: Vec<JobRow> = sqlx::query_as(&sql)
        .bind(at)
        .bind(device)
        .fetch_all(conn)
        .await?;
    rows.into_iter().map(Job::try_from).collect()
}

/// The job; its row stays locked until the connection's transaction ends.
pub(super) async fn job_locked(conn: &mut PgConnection, id: JobId) -> Result<Option<Job>, Error> {
    let sql = format!("{SELECT_JOB}WHERE id = $1 FOR UPDATE");
    let row: Option<JobRow> = sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?;
    row.map(Job::try_from).transpose()
}

/// The oldest queued jobs, locked; a job that a different transaction has locked is skipped,
/// so two claims never take one job (BLD-004).
pub(super) async fn oldest_queued(conn: &mut PgConnection) -> Result<Vec<Job>, Error> {
    let condition = format!(
        "WHERE state = 'queued' AND $1::timestamptz IS NULL AND $2::uuid IS NULL \
         ORDER BY created_at, id LIMIT {CLAIM_LOOKS_AT} FOR UPDATE SKIP LOCKED"
    );
    jobs_where(conn, &condition, None, None).await
}

/// The jobs that `device` holds, locked.
pub(super) async fn held_by(conn: &mut PgConnection, device: DeviceId) -> Result<Vec<Job>, Error> {
    let condition = "WHERE device_id = $2 AND $1::timestamptz IS NULL ORDER BY id FOR UPDATE";
    jobs_where(conn, condition, None, Some(device)).await
}

/// The jobs whose lease ended by `now`, locked; one that a report holds is skipped.
pub(super) async fn ended_leases(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
) -> Result<Vec<Job>, Error> {
    let condition = "WHERE lease_ends_at <= $1 AND $2::uuid IS NULL \
                     ORDER BY lease_ends_at, id FOR UPDATE SKIP LOCKED";
    jobs_where(conn, condition, Some(now), None).await
}

/// The held jobs whose attempt started at least its time limit and 5 minutes before `now`,
/// locked (SF-011).
pub(super) async fn over_time(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
) -> Result<Vec<Job>, Error> {
    let condition = "WHERE lease_ends_at IS NOT NULL AND $2::uuid IS NULL \
                     AND claimed_at + make_interval(secs => limit_seconds + 300) <= $1 \
                     ORDER BY claimed_at, id FOR UPDATE SKIP LOCKED";
    jobs_where(conn, condition, Some(now), None).await
}

/// Writes a new job.
pub(super) async fn insert(conn: &mut PgConnection, job: &Job) -> Result<(), Error> {
    let asked = &job.asked;
    sqlx::query(
        "INSERT INTO jobs.jobs (id, work_type, kind, os, architecture, project_id, forge, owner, \
         name, parameters, secrets, limit_seconds, limit_idle_seconds, limit_memory_bytes, \
         limit_disk_bytes, limit_orphan_seconds, state, created_at, state_changed_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
         $18, $18)",
    )
    .bind(job.id)
    .bind(word(&asked.work_type)?)
    .bind(word(&asked.kind)?)
    .bind(asked.os.as_ref().map(word).transpose()?)
    .bind(asked.architecture.as_ref().map(word).transpose()?)
    .bind(asked.project)
    .bind(&asked.repository.forge)
    .bind(&asked.repository.owner)
    .bind(&asked.repository.name)
    .bind(&asked.parameters)
    .bind(&asked.secrets)
    .bind(stored(asked.limits.seconds)?)
    .bind(stored(asked.limits.idle_seconds)?)
    .bind(stored(asked.limits.memory_bytes)?)
    .bind(stored(asked.limits.disk_bytes)?)
    .bind(stored(asked.limits.orphan_seconds)?)
    .bind(job.state.as_str())
    .bind(job.created_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Writes each part of the job that a rule can change.
pub(super) async fn save(conn: &mut PgConnection, job: &Job) -> Result<(), Error> {
    let result = job
        .result
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| Error::Unreadable)?;
    sqlx::query(
        "UPDATE jobs.jobs SET state = $2, attempt = $3, lost = $4, device_id = $5, \
         last_device_id = $6, refused_by = $7, lease_ends_at = $8, claimed_at = $9, \
         stop_reason = $10, reason = $11, result = $12, log_end = $13, state_changed_at = $14, \
         ended_at = $15 WHERE id = $1",
    )
    .bind(job.id)
    .bind(job.state.as_str())
    .bind(i32::try_from(job.attempt).map_err(|_| Error::Unreadable)?)
    .bind(i32::from(job.lost))
    .bind(job.holder)
    .bind(job.last_device)
    .bind(&job.refused_by)
    .bind(job.lease_ends_at)
    .bind(job.claimed_at)
    .bind(job.stop.as_ref().map(word).transpose()?)
    .bind(&job.reason)
    .bind(result)
    .bind(stored(job.log_end)?)
    .bind(job.changed_at)
    .bind(job.ended_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Adds one part to the log of the job and drops the oldest parts over 2 MiB.
pub(super) async fn append_log(
    conn: &mut PgConnection,
    job: JobId,
    from: u64,
    text: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO jobs.log_parts (job_id, from_byte, text) VALUES ($1, $2, $3)")
        .bind(job)
        .bind(stored(from)?)
        .bind(text)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "DELETE FROM jobs.log_parts p USING ( \
           SELECT from_byte, sum(octet_length(text)) OVER (ORDER BY from_byte DESC) AS newer \
           FROM jobs.log_parts WHERE job_id = $1 \
         ) kept WHERE p.job_id = $1 AND p.from_byte = kept.from_byte \
           AND kept.newer - octet_length(p.text) >= $2",
    )
    .bind(job)
    .bind(MAX_LOG_BYTES)
    .execute(conn)
    .await?;
    Ok(())
}

/// The log of the job that the hub holds, oldest part first.
pub(super) async fn log_of(conn: &mut PgConnection, job: JobId) -> Result<String, Error> {
    let parts: Vec<(String,)> =
        sqlx::query_as("SELECT text FROM jobs.log_parts WHERE job_id = $1 ORDER BY from_byte")
            .bind(job)
            .fetch_all(conn)
            .await?;
    Ok(parts.into_iter().map(|(text,)| text).collect())
}

type MachineRow = (
    String,
    bool,
    bool,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// What the hub knows of the machine, locked; `None` before its first claim.
pub(super) async fn machine_locked(
    conn: &mut PgConnection,
    device: DeviceId,
) -> Result<Option<Machine>, Error> {
    let row: Option<MachineRow> = sqlx::query_as(
        "SELECT switch, unclean, off_by_operator, off_until, first_on_at FROM jobs.machines \
         WHERE device_id = $1 FOR UPDATE",
    )
    .bind(device)
    .fetch_optional(conn)
    .await?;
    row.map(
        |(switch, unclean, off_by_operator, off_until, first_on_at)| {
            Ok(Machine {
                device,
                switch: read_word::<Switch>(&switch)?,
                unclean,
                off_by_operator,
                off_until,
                first_on_at,
            })
        },
    )
    .transpose()
}

/// Writes what the machine said in its claim at `now`.
pub(super) async fn save_machine(
    conn: &mut PgConnection,
    machine: &Machine,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO jobs.machines (device_id, switch, unclean, off_by_operator, off_until, \
         first_on_at, last_claim_at) VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (device_id) DO UPDATE SET switch = $2, unclean = $3, off_by_operator = $4, \
         off_until = $5, first_on_at = $6, last_claim_at = $7",
    )
    .bind(machine.device)
    .bind(word(&machine.switch)?)
    .bind(machine.unclean)
    .bind(machine.off_by_operator)
    .bind(machine.off_until)
    .bind(machine.first_on_at)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

/// Records that the machine refused or lost a job at `at`, and returns how many it refused or
/// lost since `since`. A machine with no row of its own (a device that was removed) counts 0.
pub(super) async fn add_fault(
    conn: &mut PgConnection,
    (device, fault): (DeviceId, Fault),
    at: DateTime<Utc>,
    since: DateTime<Utc>,
) -> Result<usize, Error> {
    let known: Option<(i64,)> = sqlx::query_as(
        "WITH added AS ( \
           INSERT INTO jobs.faults (device_id, at, kind) \
           SELECT $1, $2, $3 WHERE EXISTS (SELECT FROM devices.devices WHERE id = $1) \
           RETURNING 1 \
         ) SELECT (SELECT count(*) FROM added) + \
                  (SELECT count(*) FROM jobs.faults WHERE device_id = $1 AND at > $4)",
    )
    .bind(device)
    .bind(at)
    .bind(fault.as_str())
    .bind(since)
    .fetch_optional(conn)
    .await?;
    Ok(usize::try_from(known.map_or(0, |(faults,)| faults)).unwrap_or(usize::MAX))
}

/// Sets the end of the hour with no work of the machine.
pub(super) async fn set_off_until(
    conn: &mut PgConnection,
    device: DeviceId,
    until: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query("UPDATE jobs.machines SET off_until = $2 WHERE device_id = $1")
        .bind(device)
        .bind(until)
        .execute(conn)
        .await?;
    Ok(())
}
