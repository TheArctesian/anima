//! What the API reads: pages of projects, the repositories, resources and signs of a set of
//! projects in one query each (R-014), and one repository with its account for a live read.
//! I/O only.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    ids::{ProjectId, RepositoryId, ResourceId, SourceAccountId},
    work::projects::{browse::ProjectBrowse, page::After},
};

#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct ProjectRow {
    pub id: ProjectId,
    pub slug: String,
    pub name: String,
    /// The name in lower case: the first part of the page order.
    pub sort_name: String,
    pub rank: Option<i64>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one column each of the K1 fields of ProjectRepository"
)]
pub(in crate::work::projects) struct RepositoryRow {
    pub id: RepositoryId,
    pub project_id: ProjectId,
    /// `None` when its source account was removed with its records kept (ITG-004).
    pub account_id: Option<SourceAccountId>,
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

const PROJECT: &str = "p.id, p.slug, p.name, lower(p.name) AS sort_name, NULL::bigint AS rank";

// The first repository in the current view supplies the metadata for a project.
const BROWSE_FROM: &str = "FROM projects.projects p
    JOIN LATERAL (
       SELECT r.primary_language, r.size_kib, r.last_commit_at, r.repository_created_at, r.commit_count, r.stars
       FROM projects.repositories r
       WHERE r.project_id = p.id AND (r.left_out_at IS NOT NULL) = $1
       ORDER BY r.first_seen_at, r.id LIMIT 1
    ) r ON true
    WHERE (NOT $1 OR NOT EXISTS (
       SELECT 1 FROM projects.repositories kept WHERE kept.project_id = p.id AND kept.left_out_at IS NULL
    ))";

/// Up to `limit` matching projects after `after`. Unknown metadata sorts last.
pub(in crate::work::projects) async fn page(
    db: &PgPool,
    (left_out, browse): (bool, &ProjectBrowse),
    after: Option<&After>,
    limit: i64,
) -> Result<Vec<ProjectRow>, sqlx::Error> {
    let order = browse.order.unwrap_or_default();
    sqlx::query_as(&format!(
        "WITH ranked AS (
            SELECT p.id, p.slug, p.name, lower(p.name) AS sort_name,
                COALESCE(CASE $5
                    WHEN 'commit' THEN extract(epoch FROM r.last_commit_at)::bigint
                    WHEN 'created' THEN extract(epoch FROM r.repository_created_at)::bigint
                    WHEN 'count' THEN r.commit_count::bigint
                    WHEN 'stars' THEN r.stars::bigint
                END, -9223372036854775807) AS rank
            {BROWSE_FROM}
            AND ($6::text IS NULL OR r.primary_language = $6)
            AND ($7::integer IS NULL OR r.size_kib >= $7)
            AND ($8::integer IS NULL OR r.size_kib <= $8)
            AND ($10::uuid IS NULL OR p.group_id = $10)
            AND (NOT $11 OR p.group_id IS NULL)
         )
         SELECT id, slug, name, sort_name,
                CASE WHEN $5 <> 'name' THEN rank END AS rank
         FROM ranked WHERE $2::text IS NULL OR
              (-rank, sort_name COLLATE \"C\", id) >
              (-COALESCE($9::bigint, -9223372036854775807), $2 COLLATE \"C\", $3)
         ORDER BY ranked.rank DESC, sort_name COLLATE \"C\", id LIMIT $4"
    ))
    .bind(left_out)
    .bind(after.map(|a| a.name.as_str()))
    .bind(after.map(|a| a.id))
    .bind(limit)
    .bind(order.key())
    .bind(&browse.language)
    .bind(browse.min_size_kib)
    .bind(browse.max_size_kib)
    .bind(after.and_then(|a| a.rank))
    .bind(browse.group_id())
    .bind(browse.ungrouped.unwrap_or(false))
    .fetch_all(db)
    .await
}

pub(in crate::work::projects) async fn languages(
    db: &PgPool,
    left_out: bool,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(&format!(
        "SELECT DISTINCT r.primary_language {BROWSE_FROM}
         AND r.primary_language IS NOT NULL ORDER BY r.primary_language"
    ))
    .bind(left_out)
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
        "SELECT id, project_id, account_id, full_name, html_url, default_branch, private, archived, fork, \
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

/// What a live read needs of one repository: its name, the account that reads it and the
/// operator's choices.
#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct ReachRow {
    pub full_name: String,
    pub html_url: String,
    pub default_branch: Option<String>,
    pub account_id: Option<SourceAccountId>,
    pub left_out: bool,
    pub gone: bool,
}

/// The repository with this id, with its account.
pub(in crate::work::projects) async fn reach(
    db: &PgPool,
    id: RepositoryId,
) -> Result<Option<ReachRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT full_name, html_url, default_branch, account_id, \
         left_out_at IS NOT NULL AS left_out, gone_at IS NOT NULL AS gone \
         FROM projects.repositories WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}
