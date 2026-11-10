//! SQL of publishing (D-25, D-26): the job the hub owns once its files are in, the hold that
//! keeps one publisher on it, the files the upload URLs were given for and the feed folders
//! written.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{JobRow, SELECT_JOB};
use crate::{
    builds::{
        Error,
        model::{BuildJob, Target, UploadedFile},
    },
    ids::BuildJobId,
};

/// Whether the hub publishes the job now: its builder's part is over (D-25).
pub(in crate::builds) async fn is_publishing(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
) -> Result<bool, Error> {
    Ok(publish_attempt(db, job).await?.is_some())
}

/// Ends the builder's lease and gives the job to the publisher in one statement, so an
/// `uploading` job always has a lease or a publisher (D-25).
pub(in crate::builds) async fn hand_to_publisher(
    conn: &mut PgConnection,
    job: BuildJobId,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE builds.jobs SET lease_ends_at = NULL, publishing_since = $2, \
             publish_held_until = NULL, publish_attempts = 0 \
         WHERE id = $1",
    )
    .bind(job)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}

/// The job the publisher may take at `now`, the oldest first, locked until the transaction
/// ends, with the tries it has had; a job another transaction holds is skipped (D-25). A job
/// whose upload URLs end after `expired_by` waits (D-28).
pub(in crate::builds) async fn next_publishable(
    conn: &mut PgConnection,
    now: DateTime<Utc>,
    expired_by: DateTime<Utc>,
) -> Result<Option<(BuildJob, i32)>, Error> {
    let sql = format!(
        "{SELECT_JOB}WHERE j.publishing_since IS NOT NULL \
             AND (j.publish_held_until IS NULL OR j.publish_held_until <= $1) \
             AND (j.upload_urls_expire_at IS NULL OR j.upload_urls_expire_at <= $2) \
         ORDER BY j.publishing_since, j.id LIMIT 1 FOR UPDATE OF j SKIP LOCKED"
    );
    let row: Option<JobRow> = sqlx::query_as(&sql)
        .bind(now)
        .bind(expired_by)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let job = BuildJob::try_from(row)?;
    let attempts = publish_attempt(&mut *conn, job.id).await?.unwrap_or(0);
    Ok(Some((job, attempts)))
}

/// Records that the job's upload URLs work until `until`, keeping the latest end (D-28).
pub(in crate::builds) async fn extend_upload_expiry(
    conn: &mut PgConnection,
    job: BuildJobId,
    until: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE builds.jobs SET upload_urls_expire_at = \
             greatest(upload_urls_expire_at, $2) WHERE id = $1",
    )
    .bind(job)
    .bind(until)
    .execute(conn)
    .await?;
    Ok(())
}

/// Whether try `attempt` still holds the job at `now`: the hub publishes it, no later try took
/// it, and its hold has not run out (D-25). The last fence before a write.
pub(in crate::builds) async fn still_held(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
    attempt: i32,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM builds.jobs WHERE id = $1 AND state = 'uploading' \
             AND publishing_since IS NOT NULL AND publish_attempts = $2 \
             AND publish_held_until > $3)",
    )
    .bind(job)
    .bind(attempt)
    .bind(now)
    .fetch_one(db)
    .await?)
}

/// The jobs the hub was given to publish, locked, the oldest first; for a hub whose publishing
/// is off (D-25).
pub(in crate::builds) async fn handed_over(
    conn: &mut PgConnection,
) -> Result<Vec<BuildJob>, Error> {
    let sql = format!(
        "{SELECT_JOB}WHERE j.publishing_since IS NOT NULL \
         ORDER BY j.publishing_since, j.id FOR UPDATE OF j SKIP LOCKED"
    );
    let rows: Vec<JobRow> = sqlx::query_as(&sql).fetch_all(conn).await?;
    rows.into_iter().map(BuildJob::try_from).collect()
}

/// Whether a job of `target` and `version` is published, so its objects are people's.
pub(in crate::builds) async fn is_published(
    db: impl PgExecutor<'_>,
    target: Target,
    version: &str,
) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM builds.jobs WHERE target = $1 AND version = $2 \
             AND state = 'published')",
    )
    .bind(target.as_str())
    .bind(version)
    .fetch_one(db)
    .await?)
}

/// Keeps the publisher off the job until `until`; with `attempt`, records it as that try.
pub(in crate::builds) async fn hold(
    conn: &mut PgConnection,
    job: BuildJobId,
    attempt: Option<i32>,
    until: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "UPDATE builds.jobs SET publish_held_until = $2, \
             publish_attempts = coalesce($3, publish_attempts) \
         WHERE id = $1 AND publishing_since IS NOT NULL",
    )
    .bind(job)
    .bind(until)
    .bind(attempt)
    .execute(conn)
    .await?;
    Ok(())
}

/// The publisher's try on the job, or `None` when the hub does not publish it.
pub(in crate::builds) async fn publish_attempt(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
) -> Result<Option<i32>, Error> {
    let attempt: Option<Option<i32>> = sqlx::query_scalar(
        "SELECT CASE WHEN publishing_since IS NOT NULL THEN publish_attempts END \
         FROM builds.jobs WHERE id = $1",
    )
    .bind(job)
    .fetch_optional(db)
    .await?;
    Ok(attempt.flatten())
}

/// The files the job was given upload URLs for (D-26), by name.
pub(in crate::builds) async fn files_of(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
) -> Result<Vec<UploadedFile>, Error> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT name, sha256, size_bytes FROM builds.job_files WHERE job_id = $1 ORDER BY name COLLATE \"C\"",
    )
    .bind(job)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(name, sha256, size)| UploadedFile { name, sha256, size })
        .collect())
}

/// The feed the builder proposed for the job.
pub(in crate::builds) async fn feed_of(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
) -> Result<Option<String>, Error> {
    Ok(
        sqlx::query_scalar("SELECT feed FROM builds.feeds WHERE job_id = $1")
            .bind(job)
            .fetch_optional(db)
            .await?,
    )
}

/// The versions `target` has published (BLD-069, D-27).
pub(in crate::builds) async fn published_versions(
    db: impl PgExecutor<'_>,
    target: Target,
) -> Result<Vec<String>, Error> {
    Ok(sqlx::query_scalar(
        "SELECT version FROM builds.jobs WHERE target = $1 AND state = 'published'",
    )
    .bind(target.as_str())
    .fetch_all(db)
    .await?)
}

/// Records the feed folders the hub wrote for the job; empty for none (BLD-068).
pub(in crate::builds) async fn set_written(
    conn: &mut PgConnection,
    job: BuildJobId,
    folders: &[&str],
) -> Result<(), Error> {
    sqlx::query("UPDATE builds.feeds SET written = $2 WHERE job_id = $1")
        .bind(job)
        .bind(folders)
        .execute(conn)
        .await?;
    Ok(())
}
