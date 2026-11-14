//! The SQL of `sources.sync_runs`: one row per run, its counts and problems (SRC-005).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgExecutor, types::Json};

use crate::{
    engine::sources::{
        intake::Counts,
        model::{Outcome, Trigger},
    },
    ids::{SourceAccountId, SyncRunId},
};

/// How many runs of an account are kept.
const KEPT: i64 = 200;
/// How many problems one run keeps.
const MAX_PROBLEMS: i64 = 100;

/// A problem as stored: fixed words from the provider, never its reply (LOG-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredProblem {
    pub code: String,
    pub subject: String,
    pub message: String,
}

/// One stored run.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Run {
    pub id: SyncRunId,
    pub trigger: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub inserted: i32,
    pub changed: i32,
    pub unchanged: i32,
    pub removed: i32,
    pub problems: Json<Vec<StoredProblem>>,
    pub error: Option<String>,
}

/// Ends every run of the account that is still open: a newer claim took it over.
pub(in crate::engine::sources) async fn supersede(
    conn: &mut PgConnection,
    account: SourceAccountId,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sources.sync_runs SET ended_at = $2, outcome = 'stopped', \
         error = 'A newer run took over.' WHERE account_id = $1 AND ended_at IS NULL",
    )
    .bind(account)
    .bind(now)
    .execute(conn)
    .await
    .map(drop)
}

pub(in crate::engine::sources) async fn start(
    conn: &mut PgConnection,
    (id, account): (SyncRunId, SourceAccountId),
    trigger: Trigger,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sources.sync_runs (id, account_id, trigger, started_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(account)
    .bind(trigger.as_str())
    .bind(now)
    .execute(conn)
    .await
    .map(drop)
}

/// Adds a page's counts and problems to the run; problems beyond the first 100 are dropped.
pub(in crate::engine::sources) async fn add(
    conn: &mut PgConnection,
    id: SyncRunId,
    counts: Counts,
    problems: &[StoredProblem],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sources.sync_runs SET inserted = inserted + $2, changed = changed + $3, \
         unchanged = unchanged + $4, removed = removed + $5, \
         problems = coalesce((SELECT jsonb_agg(p.value ORDER BY p.ordinality) FROM \
             jsonb_array_elements(problems || $6) WITH ORDINALITY p WHERE p.ordinality <= $7), '[]') \
         WHERE id = $1",
    )
    .bind(id)
    .bind(i64::from(counts.inserted))
    .bind(i64::from(counts.changed))
    .bind(i64::from(counts.unchanged))
    .bind(i64::from(counts.removed))
    .bind(Json(problems))
    .bind(MAX_PROBLEMS)
    .execute(conn)
    .await
    .map(drop)
}

/// Ends the run, then deletes the account's runs beyond the newest 200.
pub(in crate::engine::sources) async fn finish(
    conn: &mut PgConnection,
    (id, account): (SyncRunId, SourceAccountId),
    (now, outcome): (DateTime<Utc>, Outcome),
    error: Option<&'static str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sources.sync_runs SET ended_at = $2, outcome = $3, error = $4 \
         WHERE id = $1 AND ended_at IS NULL",
    )
    .bind(id)
    .bind(now)
    .bind(outcome.as_str())
    .bind(error)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "DELETE FROM sources.sync_runs WHERE account_id = $1 AND id NOT IN \
         (SELECT id FROM sources.sync_runs WHERE account_id = $1 ORDER BY started_at DESC, id DESC LIMIT $2)",
    )
    .bind(account)
    .bind(KEPT)
    .execute(conn)
    .await
    .map(drop)
}

/// The newest `first` runs of the account, newest first.
pub(in crate::engine::sources) async fn newest(
    db: impl PgExecutor<'_>,
    account: SourceAccountId,
    first: i64,
) -> Result<Vec<Run>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, trigger, started_at, ended_at, outcome, inserted, changed, unchanged, \
         removed, problems, error FROM sources.sync_runs WHERE account_id = $1 \
         ORDER BY started_at DESC, id DESC LIMIT $2",
    )
    .bind(account)
    .bind(first)
    .fetch_all(db)
    .await
}
