//! SQL on `builds.log_lines`, `builds.job_files` and `builds.feeds`: what a builder sends
//! with its reports.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use crate::{
    builds::{
        Error,
        model::{LogLine, UploadedFile},
    },
    ids::BuildJobId,
};

/// Stores `lines` with the sequence numbers that follow `last`.
pub(in crate::builds) async fn append_lines(
    conn: &mut PgConnection,
    job: BuildJobId,
    last: i64,
    lines: &[String],
) -> Result<(), Error> {
    if lines.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO builds.log_lines (job_id, sequence, text) \
         SELECT $1, $2 + u.ord, u.text FROM unnest($3::text[]) WITH ORDINALITY AS u(text, ord)",
    )
    .bind(job)
    .bind(last)
    .bind(lines)
    .execute(conn)
    .await?;
    Ok(())
}

/// Deletes the job's oldest lines until its log holds at most `max_bytes` (BLD-048, D-4).
pub(in crate::builds) async fn trim_log(
    conn: &mut PgConnection,
    job: BuildJobId,
    max_bytes: i64,
) -> Result<(), Error> {
    sqlx::query(
        "DELETE FROM builds.log_lines l USING ( \
             SELECT sequence, sum(octet_length(text)) OVER (ORDER BY sequence DESC) AS kept \
             FROM builds.log_lines WHERE job_id = $1 \
         ) newer \
         WHERE l.job_id = $1 AND l.sequence = newer.sequence AND newer.kept > $2",
    )
    .bind(job)
    .bind(max_bytes)
    .execute(conn)
    .await?;
    Ok(())
}

/// The job's stored log, oldest line first.
pub(in crate::builds) async fn lines(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
) -> Result<Vec<LogLine>, Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT sequence, text FROM builds.log_lines WHERE job_id = $1 ORDER BY sequence",
    )
    .bind(job)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(sequence, text)| LogLine { sequence, text })
        .collect())
}

/// Makes `files` the job's uploaded files, replacing an earlier report.
pub(in crate::builds) async fn replace_files(
    conn: &mut PgConnection,
    job: BuildJobId,
    files: &[UploadedFile],
) -> Result<(), Error> {
    sqlx::query("DELETE FROM builds.job_files WHERE job_id = $1")
        .bind(job)
        .execute(&mut *conn)
        .await?;
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    let hashes: Vec<&str> = files.iter().map(|f| f.sha256.as_str()).collect();
    let sizes: Vec<i64> = files.iter().map(|f| f.size).collect();
    sqlx::query(
        "INSERT INTO builds.job_files (job_id, name, sha256, size_bytes) \
         SELECT $1, u.name, u.sha256, u.size FROM unnest($2::text[], $3::text[], $4::bigint[]) \
             AS u(name, sha256, size)",
    )
    .bind(job)
    .bind(names)
    .bind(hashes)
    .bind(sizes)
    .execute(conn)
    .await?;
    Ok(())
}

/// Keeps the proposed feed of the job, replacing an earlier one (D-5).
pub(in crate::builds) async fn set_feed(
    conn: &mut PgConnection,
    job: BuildJobId,
    feed: &str,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO builds.feeds (job_id, feed, reported_at) VALUES ($1, $2, $3) \
         ON CONFLICT (job_id) DO UPDATE SET feed = excluded.feed, \
             reported_at = excluded.reported_at",
    )
    .bind(job)
    .bind(feed)
    .bind(now)
    .execute(conn)
    .await?;
    Ok(())
}
