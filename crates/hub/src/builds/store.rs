//! All SQL on schema `builds`: I/O only (CQ-032). Ids and time come in as parameters. Writes
//! take the caller's connection, so a change commits with its audit entry or event. Jobs are
//! here; releases, builders and logs are in the submodules.

mod builders;
mod logs;
mod releases;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

pub(super) use builders::{builder_by_token, builders, delete_builder, insert_builder, touch};
pub(super) use logs::{append_lines, lines, replace_files, set_feed, trim_log};
pub(super) use releases::{built, insert_release, jobs_of, lock_releases, releases_after};

use super::{
    Error,
    model::{BuildJob, Commit, JobState, StateTimes, Target},
};
use crate::ids::{BuildJobId, BuilderId, ReleaseId};

const SELECT_JOB: &str = "SELECT j.id, j.release_id, j.target, j.version, r.commit_sha, j.state, \
                                 j.failure_reason, j.attempts, j.builder_id, j.builder_name, \
                                 j.lease_ends_at, j.log_sequence, j.created_at, j.queued_at, \
                                 j.claimed_at, j.building_at, j.uploading_at, j.published_at, \
                                 j.failed_at, j.cancelled_at, j.state_changed_at \
                          FROM builds.jobs j JOIN builds.releases r ON r.id = j.release_id ";

#[derive(sqlx::FromRow)]
struct JobRow {
    id: BuildJobId,
    release_id: ReleaseId,
    target: String,
    version: String,
    commit_sha: String,
    state: String,
    failure_reason: Option<String>,
    attempts: i32,
    builder_id: Option<BuilderId>,
    builder_name: Option<String>,
    lease_ends_at: Option<DateTime<Utc>>,
    log_sequence: i64,
    created_at: DateTime<Utc>,
    queued_at: DateTime<Utc>,
    claimed_at: Option<DateTime<Utc>>,
    building_at: Option<DateTime<Utc>>,
    uploading_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    state_changed_at: DateTime<Utc>,
}

impl TryFrom<JobRow> for BuildJob {
    type Error = Error;

    fn try_from(row: JobRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            release: row.release_id,
            target: Target::from_stored(&row.target).ok_or(Error::Unreadable)?,
            version: row.version.parse().map_err(|_| Error::Unreadable)?,
            commit: Commit(row.commit_sha),
            state: JobState::from_stored(&row.state).ok_or(Error::Unreadable)?,
            failure_reason: row.failure_reason,
            attempts: u8::try_from(row.attempts).map_err(|_| Error::Unreadable)?,
            builder: row.builder_id,
            builder_name: row.builder_name,
            lease_ends_at: row.lease_ends_at,
            log_sequence: row.log_sequence,
            created_at: row.created_at,
            times: StateTimes {
                queued: row.queued_at,
                claimed: row.claimed_at,
                building: row.building_at,
                uploading: row.uploading_at,
                published: row.published_at,
                failed: row.failed_at,
                cancelled: row.cancelled_at,
                changed: row.state_changed_at,
            },
        })
    }
}

fn read_all(rows: Vec<JobRow>) -> Result<Vec<BuildJob>, Error> {
    rows.into_iter().map(BuildJob::try_from).collect()
}

/// The job; its row stays locked until the connection's transaction ends.
pub(super) async fn job_locked(
    conn: &mut PgConnection,
    id: BuildJobId,
) -> Result<Option<BuildJob>, Error> {
    let sql = format!("{SELECT_JOB}WHERE j.id = $1 FOR UPDATE OF j");
    let row: Option<JobRow> = sqlx::query_as(&sql).bind(id).fetch_optional(conn).await?;
    row.map(BuildJob::try_from).transpose()
}

/// The oldest queued job of one of `targets`, locked; a job another transaction has locked is
/// skipped, so two claims never take one job (BLD-004, D-3).
pub(super) async fn oldest_queued(
    conn: &mut PgConnection,
    targets: &[Target],
) -> Result<Option<BuildJob>, Error> {
    let targets: Vec<&str> = targets.iter().map(|t| t.as_str()).collect();
    let sql = format!(
        "{SELECT_JOB}WHERE j.state = 'queued' AND j.target = ANY($1) \
         ORDER BY j.created_at, j.id LIMIT 1 FOR UPDATE OF j SKIP LOCKED"
    );
    let row: Option<JobRow> = sqlx::query_as(&sql)
        .bind(targets)
        .fetch_optional(conn)
        .await?;
    row.map(BuildJob::try_from).transpose()
}

/// Every job whose lease ended at or before `now`, locked; one another transaction is changing
/// is left to it (D-2).
pub(super) async fn ended_leases(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
) -> Result<Vec<BuildJob>, Error> {
    let sql = format!(
        "{SELECT_JOB}WHERE j.lease_ends_at <= $1 ORDER BY j.lease_ends_at, j.id \
         FOR UPDATE OF j SKIP LOCKED"
    );
    let rows: Vec<JobRow> = sqlx::query_as(&sql).bind(now).fetch_all(conn).await?;
    read_all(rows)
}

/// Whether the builder holds a lease that has not ended at `now` (BLD-029).
pub(super) async fn holds_lease(
    db: impl PgExecutor<'_>,
    builder: BuilderId,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM builds.jobs WHERE builder_id = $1 AND lease_ends_at > $2)",
    )
    .bind(builder)
    .bind(now)
    .fetch_one(db)
    .await?)
}

pub(super) async fn insert_job(conn: &mut PgConnection, job: &BuildJob) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO builds.jobs (id, release_id, target, version, state, created_at, \
                                  queued_at, state_changed_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(job.id)
    .bind(job.release)
    .bind(job.target.as_str())
    .bind(job.version.to_string())
    .bind(job.state.as_str())
    .bind(job.created_at)
    .bind(job.times.queued)
    .bind(job.times.changed)
    .execute(conn)
    .await?;
    Ok(())
}

/// Writes every field of the job that a change can touch.
pub(super) async fn save(conn: &mut PgConnection, job: &BuildJob) -> Result<(), Error> {
    sqlx::query(
        "UPDATE builds.jobs SET state = $2, failure_reason = $3, attempts = $4, builder_id = $5, \
             builder_name = $6, lease_ends_at = $7, log_sequence = $8, queued_at = $9, \
             claimed_at = $10, building_at = $11, uploading_at = $12, published_at = $13, \
             failed_at = $14, cancelled_at = $15, state_changed_at = $16 \
         WHERE id = $1",
    )
    .bind(job.id)
    .bind(job.state.as_str())
    .bind(job.failure_reason.as_deref())
    .bind(i32::from(job.attempts))
    .bind(job.builder)
    .bind(job.builder_name.as_deref())
    .bind(job.lease_ends_at)
    .bind(job.log_sequence)
    .bind(job.times.queued)
    .bind(job.times.claimed)
    .bind(job.times.building)
    .bind(job.times.uploading)
    .bind(job.times.published)
    .bind(job.times.failed)
    .bind(job.times.cancelled)
    .bind(job.times.changed)
    .execute(conn)
    .await?;
    Ok(())
}
