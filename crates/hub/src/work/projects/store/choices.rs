//! The operator's choices: leave a repository out, restore it, confirm a resource (PRO-002,
//! PRO-004). I/O only.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::ids::{ProjectId, RepositoryId, ResourceId, SourceAccountId};

/// A repository as a choice about it reads it.
#[derive(Clone, Copy, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct Chosen {
    pub project_id: ProjectId,
    pub account_id: Option<SourceAccountId>,
    pub left_out: bool,
}

/// The repository `id`, locked until the transaction ends.
pub(in crate::work::projects) async fn repository(
    conn: &mut PgConnection,
    id: RepositoryId,
) -> Result<Option<Chosen>, sqlx::Error> {
    sqlx::query_as(
        "SELECT project_id, account_id, left_out_at IS NOT NULL AS left_out \
         FROM projects.repositories WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

/// Sets the repository left out at `at`, or not left out when `at` is `None`.
pub(in crate::work::projects) async fn set_left_out(
    conn: &mut PgConnection,
    id: RepositoryId,
    at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects.repositories SET left_out_at = $2 WHERE id = $1")
        .bind(id)
        .bind(at)
        .execute(conn)
        .await
        .map(drop)
}

/// The project, the kind and the state of resource `id`, locked until the transaction ends.
pub(in crate::work::projects) async fn resource(
    conn: &mut PgConnection,
    id: ResourceId,
) -> Result<Option<(ProjectId, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT project_id, kind, state FROM projects.resources WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

/// Confirms resource `id` as the operator's, at `at`.
pub(in crate::work::projects) async fn confirm(
    conn: &mut PgConnection,
    id: ResourceId,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE projects.resources SET state = 'confirmed', confirmed_at = $2, \
         confirmed_by = 'operator' WHERE id = $1",
    )
    .bind(id)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

/// The slug of project `id`.
pub(in crate::work::projects) async fn slug(
    conn: &mut PgConnection,
    id: ProjectId,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT slug FROM projects.projects WHERE id = $1")
        .bind(id)
        .fetch_one(conn)
        .await
}
