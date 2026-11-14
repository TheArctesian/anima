//! What the API reads: pages of projects, and the repositories, resources and signs of a set
//! of projects in one query each (R-014). I/O only.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    ids::{ProjectId, RepositoryId, ResourceId},
    work::projects::page::After,
};

#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct ProjectRow {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    /// The name in lower case: the first part of the page order.
    pub sort_name: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one column each of the K1 fields of ProjectRepository"
)]
pub(in crate::work::projects) struct RepositoryRow {
    pub id: RepositoryId,
    pub project_id: ProjectId,
    pub full_name: String,
    pub html_url: String,
    pub default_branch: Option<String>,
    pub private: bool,
    pub archived: bool,
    pub fork: bool,
    pub description: Option<String>,
    pub pushed_at: Option<DateTime<Utc>>,
    pub left_out: bool,
    pub gone: bool,
    pub listed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct ResourceRow {
    pub id: ResourceId,
    pub project_id: ProjectId,
    pub kind: String,
    pub state: String,
    pub found_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// A sign of a repository of a project that is not left out: `(project, sign, path)`.
pub(in crate::work::projects) type EvidenceRow = (ProjectId, String, Option<String>);

const PROJECT: &str = "p.id, p.slug, p.name, lower(p.name) AS sort_name";

/// Up to `limit` projects after `after`, by lower-case name then id. `left_out` false: those
/// with a repository that is not left out; true: those whose repositories are all left out.
pub(in crate::work::projects) async fn page(
    db: &PgPool,
    left_out: bool,
    after: Option<&After>,
    limit: i64,
) -> Result<Vec<ProjectRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {PROJECT} FROM projects.projects p \
         WHERE EXISTS (SELECT 1 FROM projects.repositories r WHERE r.project_id = p.id) \
         AND EXISTS (SELECT 1 FROM projects.repositories r \
                     WHERE r.project_id = p.id AND r.left_out_at IS NULL) <> $1 \
         AND ($2::text IS NULL OR (lower(p.name) COLLATE \"C\", p.id) > ($2 COLLATE \"C\", $3)) \
         ORDER BY lower(p.name) COLLATE \"C\", p.id LIMIT $4"
    ))
    .bind(left_out)
    .bind(after.map(|a| a.name.as_str()))
    .bind(after.map(|a| a.id))
    .bind(limit)
    .fetch_all(db)
    .await
}

/// The project with this slug.
pub(in crate::work::projects) async fn by_slug(
    db: &PgPool,
    slug: &str,
) -> Result<Option<ProjectRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {PROJECT} FROM projects.projects p WHERE p.slug = $1"
    ))
    .bind(slug)
    .fetch_optional(db)
    .await
}

/// The project with this id.
pub(in crate::work::projects) async fn by_id(
    db: &PgPool,
    id: ProjectId,
) -> Result<Option<ProjectRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {PROJECT} FROM projects.projects p WHERE p.id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await
}

/// The repositories of `projects`, each project's first one first.
pub(in crate::work::projects) async fn repositories(
    db: &PgPool,
    projects: &[ProjectId],
) -> Result<Vec<RepositoryRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, project_id, full_name, html_url, default_branch, private, archived, fork, \
         description, pushed_at, left_out_at IS NOT NULL AS left_out, \
         gone_at IS NOT NULL AS gone, listed_at \
         FROM projects.repositories WHERE project_id = ANY($1) \
         ORDER BY project_id, first_seen_at, id",
    )
    .bind(projects)
    .fetch_all(db)
    .await
}

/// The resources of `projects`.
pub(in crate::work::projects) async fn resources(
    db: &PgPool,
    projects: &[ProjectId],
) -> Result<Vec<ResourceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, project_id, kind, state, found_at, confirmed_at FROM projects.resources \
         WHERE project_id = ANY($1) ORDER BY project_id, found_at, id",
    )
    .bind(projects)
    .fetch_all(db)
    .await
}

/// The signs of the repositories of `projects` that are not left out, each once.
pub(in crate::work::projects) async fn evidence(
    db: &PgPool,
    projects: &[ProjectId],
) -> Result<Vec<EvidenceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT DISTINCT r.project_id, s.sign, s.path FROM projects.signs s \
         JOIN projects.repositories r ON r.id = s.repository_id \
         WHERE r.project_id = ANY($1) AND r.left_out_at IS NULL \
         ORDER BY r.project_id, s.sign, s.path",
    )
    .bind(projects)
    .fetch_all(db)
    .await
}
