//! Builds: the release queue, the builders that claim its jobs under a lease, their logs, and
//! the publishing of what they built (spec Section 3.10, BLD-001 to BLD-082; WP-36 D-1 to D-5,
//! D-24 to D-27).
//!
//! Operator writes take the caller's connection, so the GraphQL layer adds the audit entry in
//! the same transaction (LOG-005). Builder operations run their own transactions. Messages in
//! [`Error`] hold no token and no log text (LOG-007).

mod builders;
pub mod graphql;
mod hidden_text;
mod jobs;
mod log_lines;
mod model;
mod page;
mod private_keys;
mod publishing;
mod queue;
mod release;
mod store;
#[cfg(test)]
mod tests;
mod updates;
mod upload;
mod version;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub use builders::{
    BuilderInput, Enrolment, Removal, audit_removed, builder_for, builders, enrol, revoke,
};
pub use jobs::{BuildQueue, Upload, claim, expire_leases, report, report_uploaded};
pub use model::{
    Arch, BuildJob, Builder, Commit, FieldError, JobState, JobUpdate, LogLine, Missing, Os,
    Platform, Release, Report, StateTimes, Target, UploadedFile,
};
pub use page::{ReleasePage, cursor};
pub use publishing::{
    Bucket, BucketError, Grant, PUBLISH_SWEEP, UploadUrl, publish_ready, publishing_off,
    upload_urls,
};
pub use release::ReleaseInput;
pub use updates::Updates;
pub use version::Version;

use crate::{
    access,
    api_errors::field_names,
    events::{self, Operator},
    ids::{BuildJobId, ReleaseId},
};

/// How often the upkeep job ends the leases that ran out (D-2).
pub const LEASE_SWEEP: std::time::Duration = std::time::Duration::from_secs(30);

/// Why a build operation failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0} not found")]
    NotFound(Missing),
    #[error("input is not valid: {}", field_names(.0))]
    Validation(Vec<FieldError>),
    /// The job is not in a state that permits the change (BLD-008, BLD-010).
    #[error("{0}")]
    Conflict(String),
    /// The builder may not do this: it holds no lease, has no record or calls from another
    /// address (BLD-010, BLD-024).
    #[error("{0}")]
    Forbidden(&'static str),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    /// A stored row is not in a form this version reads.
    #[error("a stored build row cannot be read")]
    Unreadable,
    #[error("the builder token cannot be issued or revoked")]
    Access(#[source] access::Error),
    #[error("the event log cannot be written")]
    Events(#[from] events::Error),
}

impl Error {
    /// For the process log: what failed, as a fixed word; `None` for refusals the caller
    /// caused (LOG-007, API-010).
    #[must_use]
    pub const fn fault(&self) -> Option<&'static str> {
        match self {
            Self::NotFound(_) | Self::Validation(_) | Self::Conflict(_) | Self::Forbidden(_) => {
                None
            }
            Self::Database(_) => Some("database"),
            Self::Unreadable => Some("unreadable"),
            Self::Access(_) => Some("access"),
            Self::Events(_) => Some("events"),
        }
    }
}

/// Queues a release: one `queued` job for each selected target (BLD-001). Refuses a version
/// that is not above each target's published versions, or that a target already builds
/// (BLD-002). The caller commits it with its audit entry.
///
/// # Errors
///
/// `Validation` (`version`, `commit`, `targets`) when the input breaks a rule.
pub async fn create_release(
    conn: &mut PgConnection,
    operator: &Operator,
    input: &ReleaseInput,
    now: DateTime<Utc>,
) -> Result<Release, Error> {
    let new = release::validate(input).map_err(Error::Validation)?;
    // One release is created at a time, so two cannot both pass the version check.
    store::lock_releases(conn).await?;
    let built = store::built(&mut *conn).await?;
    release::check_new(new.version, &new.targets, &built)
        .map_err(|e| Error::Validation(vec![e]))?;
    let id = ReleaseId::from(Uuid::now_v7());
    let jobs = new
        .targets
        .iter()
        .map(|target| new_job(id, *target, &new, now))
        .collect();
    let release = Release {
        id,
        version: new.version,
        commit: new.commit,
        created_by: operator.name.clone(),
        created_at: now,
        jobs,
    };
    store::insert_release(conn, &release, operator.id).await?;
    for job in &release.jobs {
        store::insert_job(conn, job).await?;
    }
    Ok(release)
}

fn new_job(
    release: ReleaseId,
    target: Target,
    new: &release::NewRelease,
    now: DateTime<Utc>,
) -> BuildJob {
    BuildJob {
        id: BuildJobId::from(Uuid::now_v7()),
        release,
        target,
        version: new.version,
        commit: new.commit.clone(),
        state: JobState::Queued,
        failure_reason: None,
        attempts: 0,
        builder: None,
        builder_name: None,
        lease_ends_at: None,
        log_sequence: 0,
        created_at: now,
        times: StateTimes {
            queued: now,
            claimed: None,
            building: None,
            uploading: None,
            published: None,
            failed: None,
            cancelled: None,
            changed: now,
        },
    }
}

/// Cancels a queued job (BLD-008). The caller commits it with its audit entry.
///
/// # Errors
///
/// `NotFound` for no such job; `Conflict` when it left the queue.
pub async fn cancel_job(
    conn: &mut PgConnection,
    id: BuildJobId,
    now: DateTime<Utc>,
) -> Result<BuildJob, Error> {
    let job = store::job_locked(conn, id)
        .await?
        .ok_or(Error::NotFound(Missing::Job(id)))?;
    let cancelled = queue::cancel(&job, now)
        .map_err(|_| Error::Conflict("Only a queued build job can be cancelled.".to_owned()))?;
    store::save(conn, &cancelled).await?;
    Ok(cancelled)
}

/// One page of releases, newest first, each with its jobs (API-005, API-006, BLD-080).
///
/// # Errors
///
/// `Validation` (`first`, `after`) for a bad page size or cursor; `Database` otherwise.
pub async fn releases(db: &PgPool, first: i32, after: Option<&str>) -> Result<ReleasePage, Error> {
    let size = page::page_size(first).map_err(|e| Error::Validation(vec![e]))?;
    let after = after
        .map(page::position)
        .transpose()
        .map_err(|e| Error::Validation(vec![e]))?;
    let limit = i64::try_from(size + 1).unwrap_or(i64::MAX);
    let mut page = page::split(store::releases_after(db, after, limit).await?, size);
    let ids: Vec<ReleaseId> = page.releases.iter().map(|r| r.id).collect();
    let mut jobs = store::jobs_of(db, &ids).await?;
    for release in &mut page.releases {
        let (mine, rest) = jobs.into_iter().partition(|job| job.release == release.id);
        release.jobs = mine;
        jobs = rest;
    }
    Ok(page)
}

/// The version the release form proposes: the highest version published or under way with the
/// patch plus 1; `None` before the first release.
///
/// # Errors
///
/// `Database` when the query fails.
pub async fn next_version(db: &PgPool) -> Result<Option<Version>, Error> {
    Ok(release::next_version(&store::built(db).await?))
}

/// The most log lines one `buildJobLog` page returns.
pub const MAX_LOG_PAGE: i32 = 1000;

/// Up to `first` stored lines of a job after sequence `after` (from its start when `None`),
/// oldest first (BLD-048, BLD-081).
///
/// # Errors
///
/// `Validation` (`first`) outside 1 to [`MAX_LOG_PAGE`]; `Database` when the query fails.
pub async fn job_log(
    db: &PgPool,
    id: BuildJobId,
    after: Option<i64>,
    first: i32,
) -> Result<Vec<LogLine>, Error> {
    if !(1..=MAX_LOG_PAGE).contains(&first) {
        let message = format!("Ask for 1 to {MAX_LOG_PAGE} lines.");
        return Err(Error::Validation(vec![FieldError::new("first", message)]));
    }
    store::lines(db, id, after.unwrap_or(0), i64::from(first)).await
}
