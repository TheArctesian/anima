//! SQL on `builds.log_lines`, `builds.job_files`, `builds.feeds` and the log state of
//! `builds.jobs`: what a builder sends with its reports.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};

use crate::{
    fleet::builds::{
        Error,
        model::{LogLine, UploadedFile},
    },
    ids::BuildJobId,
};

/// Stores `lines` with the sequence numbers that follow `last`.
pub(in crate::fleet::builds) async fn append_lines(
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

/// The private key block the job's last report left open: how many more lines it may take.
pub(in crate::fleet::builds) async fn open_key_block(
    conn: &mut PgConnection,
    job: BuildJobId,
) -> Result<Option<u16>, Error> {
    let left: Option<Option<i32>> =
        sqlx::query_scalar("SELECT log_key_lines_left FROM builds.jobs WHERE id = $1")
            .bind(job)
            .fetch_optional(conn)
            .await?;
    Ok(left.flatten().and_then(|left| u16::try_from(left).ok()))
}

/// Records the private key block the job's last report left open, if any.
pub(in crate::fleet::builds) async fn set_open_key_block(
    conn: &mut PgConnection,
    job: BuildJobId,
    left: Option<u16>,
) -> Result<(), Error> {
    sqlx::query("UPDATE builds.jobs SET log_key_lines_left = $2 WHERE id = $1")
        .bind(job)
        .bind(left.map(i32::from))
        .execute(conn)
        .await?;
    Ok(())
}

/// Deletes the job's oldest lines until its log holds at most `max_bytes`, each line counting
/// its text and `overhead` (BLD-048, D-4).
pub(in crate::fleet::builds) async fn trim_log(
    conn: &mut PgConnection,
    job: BuildJobId,
    max_bytes: i64,
    overhead: i64,
) -> Result<(), Error> {
    sqlx::query(
        "DELETE FROM builds.log_lines l USING ( \
             SELECT sequence, \
                 sum(octet_length(text) + $3) OVER (ORDER BY sequence DESC) AS kept \
             FROM builds.log_lines WHERE job_id = $1 \
         ) newer \
         WHERE l.job_id = $1 AND l.sequence = newer.sequence AND newer.kept > $2",
    )
    .bind(job)
    .bind(max_bytes)
    .bind(overhead)
    .execute(conn)
    .await?;
    Ok(())
}

/// At most `first` of the job's stored lines after sequence `after`, oldest first.
pub(in crate::fleet::builds) async fn lines(
    db: impl PgExecutor<'_>,
    job: BuildJobId,
    after: i64,
    first: i64,
) -> Result<Vec<LogLine>, Error> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT sequence, text FROM builds.log_lines WHERE job_id = $1 AND sequence > $2 \
         ORDER BY sequence LIMIT $3",
    )
    .bind(job)
    .bind(after)
    .bind(first)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(sequence, text)| LogLine { sequence, text })
        .collect())
}

/// Makes `files` the job's uploaded files, replacing an earlier report.
pub(in crate::fleet::builds) async fn replace_files(
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
pub(in crate::fleet::builds) async fn set_feed(
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
