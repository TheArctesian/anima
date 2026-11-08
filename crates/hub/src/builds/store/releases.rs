//! SQL on `builds.releases`, and the job reads that span releases.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use super::{JobRow, SELECT_JOB, read_all};
use crate::{
    builds::{
        Error,
        model::{BuildJob, Commit, JobState, Release, Target},
        release::Built,
    },
    ids::ReleaseId,
};

#[derive(sqlx::FromRow)]
struct ReleaseRow {
    id: ReleaseId,
    version: String,
    commit_sha: String,
    created_by_name: String,
    created_at: DateTime<Utc>,
}

/// The release row; the jobs go in with `insert_job`.
pub(in crate::builds) async fn insert_release(
    conn: &mut PgConnection,
    release: &Release,
    created_by: crate::ids::OperatorId,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO builds.releases \
             (id, version, commit_sha, created_by, created_by_name, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(release.id)
    .bind(release.version.to_string())
    .bind(release.commit.as_str())
    .bind(created_by)
    .bind(&release.created_by)
    .bind(release.created_at)
    .execute(conn)
    .await?;
    Ok(())
}

/// Takes the release lock until the transaction ends, so two new releases are checked one
/// after the other (BLD-002).
pub(in crate::builds) async fn lock_releases(conn: &mut PgConnection) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('builds:releases', 0))")
        .execute(conn)
        .await?;
    Ok(())
}

/// Every job that holds its version: published, queued or under way (BLD-002).
pub(in crate::builds) async fn built(db: impl PgExecutor<'_>) -> Result<Vec<Built>, Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT target, version, state FROM builds.jobs \
         WHERE state NOT IN ('failed', 'cancelled')",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(target, version, state)| {
            Ok(Built {
                target: Target::from_stored(&target).ok_or(Error::Unreadable)?,
                version: version.parse().map_err(|_| Error::Unreadable)?,
                state: JobState::from_stored(&state).ok_or(Error::Unreadable)?,
            })
        })
        .collect()
}

/// Up to `limit` releases, newest first, after the release at `after` (API-006). Their
/// `jobs` are empty; [`jobs_of`] reads them.
pub(in crate::builds) async fn releases_after(
    db: impl PgExecutor<'_>,
    after: Option<(DateTime<Utc>, ReleaseId)>,
    limit: i64,
) -> Result<Vec<Release>, Error> {
    let (at, id) = after.unzip();
    let rows: Vec<ReleaseRow> = sqlx::query_as(
        "SELECT id, version, commit_sha, created_by_name, created_at FROM builds.releases \
         WHERE $1::timestamptz IS NULL OR (created_at, id) < ($1, $2) \
         ORDER BY created_at DESC, id DESC LIMIT $3",
    )
    .bind(at)
    .bind(id)
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Release {
                id: row.id,
                version: row.version.parse().map_err(|_| Error::Unreadable)?,
                commit: Commit(row.commit_sha),
                created_by: row.created_by_name,
                created_at: row.created_at,
                jobs: Vec::new(),
            })
        })
        .collect()
}

/// The jobs of these releases in one query (API-005), each release's in target order.
pub(in crate::builds) async fn jobs_of(
    db: impl PgExecutor<'_>,
    releases: &[ReleaseId],
) -> Result<Vec<BuildJob>, Error> {
    let sql = format!("{SELECT_JOB}WHERE j.release_id = ANY($1) ORDER BY j.release_id, j.target");
    let rows: Vec<JobRow> = sqlx::query_as(&sql).bind(releases).fetch_all(db).await?;
    read_all(rows)
}
