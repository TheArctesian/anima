//! The log of a job in schema `jobs`.

use sqlx::PgConnection;

use super::stored;
use crate::{ids::JobId, work::jobs::Error};

/// The most bytes of the log of one job that the hub keeps (K10 Section 7).
const MAX_LOG_BYTES: i64 = 2 * 1024 * 1024;

/// Adds one part to the log of the job and drops the oldest parts over 2 MiB.
pub(in crate::work::jobs) async fn append_log(
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
pub(in crate::work::jobs) async fn log_of(
    conn: &mut PgConnection,
    job: JobId,
) -> Result<String, Error> {
    let parts: Vec<(String,)> =
        sqlx::query_as("SELECT text FROM jobs.log_parts WHERE job_id = $1 ORDER BY from_byte")
            .bind(job)
            .fetch_all(conn)
            .await?;
    Ok(parts.into_iter().map(|(text,)| text).collect())
}
