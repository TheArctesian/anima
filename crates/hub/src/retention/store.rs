//! All SQL on schema `retention`: I/O only.

use sqlx::{PgConnection, PgPool};

use super::model::Subject;

/// Every stored row: the subject's text and its days.
pub(super) async fn all(db: &PgPool) -> Result<Vec<(String, i32)>, sqlx::Error> {
    sqlx::query_as("SELECT subject, days FROM retention.periods ORDER BY subject")
        .fetch_all(db)
        .await
}

/// Stores `days` for `subject` and returns the days it replaced; `None` when the subject has
/// no row. The row is locked while it is read, so two changes cannot both claim the same
/// previous value.
pub(super) async fn set(
    conn: &mut PgConnection,
    subject: Subject,
    days: u16,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "WITH old AS (SELECT days FROM retention.periods WHERE subject = $1 FOR UPDATE) \
         UPDATE retention.periods p SET days = $2 FROM old WHERE p.subject = $1 \
         RETURNING old.days",
    )
    .bind(subject.as_str())
    .bind(i32::from(days))
    .fetch_optional(conn)
    .await
}
