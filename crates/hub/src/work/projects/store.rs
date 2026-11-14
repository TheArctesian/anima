//! All SQL on schema `projects`: I/O only. This file holds the writes of the import; the
//! signs and resources are in `store/signs.rs`, the operator's choices in `store/choices.rs`
//! and the reads of the API in `store/reads.rs`.

pub(super) mod choices;
pub(super) mod reads;
pub(super) mod signs;

use chrono::{DateTime, Utc};
use provider_github::Repository;
use sqlx::{PgConnection, types::Json};
use uuid::Uuid;

use super::model::Known;
use crate::ids::{ProjectId, RepositoryId, SourceAccountId};

/// The forge of every repository this slice imports.
pub(super) const FORGE: &str = "github";

const KNOWN: &str = "id, project_id, account_id, full_name, html_url, default_branch, \
    private, archived, fork, description, pushed_at, left_out_at IS NOT NULL AS left_out, \
    gone_at IS NOT NULL AS gone";

/// The stored repository with GitHub id `external_id`, locked until the transaction ends.
pub(super) async fn known(
    conn: &mut PgConnection,
    external_id: &str,
) -> Result<Option<Known>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {KNOWN} FROM projects.repositories \
         WHERE forge = $1 AND external_id = $2 FOR UPDATE"
    ))
    .bind(FORGE)
    .bind(external_id)
    .fetch_optional(conn)
    .await
}

/// The slugs that start with `stem`: the ones a new slug must not repeat.
pub(super) async fn slugs_like(
    conn: &mut PgConnection,
    stem: &str,
) -> Result<Vec<String>, sqlx::Error> {
    // A stem holds only letters, digits and dashes, so it has no LIKE wildcard.
    sqlx::query_scalar("SELECT slug FROM projects.projects WHERE slug = $1 OR slug LIKE $1 || '-%'")
        .bind(stem)
        .fetch_all(conn)
        .await
}

/// A new project `(slug, name)` holding the new repository `repository` of `account`.
pub(super) async fn import(
    conn: &mut PgConnection,
    (slug, name): (&str, &str),
    (account, repository): (SourceAccountId, &Repository),
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let project = ProjectId::from(Uuid::now_v7());
    sqlx::query(
        "INSERT INTO projects.projects (id, slug, name, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(project)
    .bind(slug)
    .bind(name)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO projects.repositories (id, project_id, account_id, forge, external_id, \
         full_name, html_url, default_branch, private, archived, fork, description, pushed_at, \
         original, first_seen_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $15)",
    )
    .bind(RepositoryId::from(Uuid::now_v7()))
    .bind(project)
    .bind(account)
    .bind(FORGE)
    .bind(&repository.external_id)
    .bind(&repository.full_name)
    .bind(&repository.html_url)
    .bind(&repository.default_branch)
    .bind(repository.private)
    .bind(repository.archived)
    .bind(repository.fork)
    .bind(&repository.description)
    .bind(repository.pushed_at)
    .bind(Json(&repository.original))
    .bind(now)
    .execute(conn)
    .await
    .map(drop)
}

/// Writes the listed fields of the known repository `id`, clears `gone`, and gives it to
/// `account` when it has no account. Its project takes the new name when it is the project's
/// first repository (PRJ-016).
pub(super) async fn refresh(
    conn: &mut PgConnection,
    (id, account): (RepositoryId, SourceAccountId),
    (repository, name): (&Repository, &str),
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE projects.repositories SET full_name = $2, html_url = $3, default_branch = $4, \
         private = $5, archived = $6, fork = $7, description = $8, pushed_at = $9, \
         original = $10, gone_at = NULL, account_id = coalesce(account_id, $11), updated_at = $12 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&repository.full_name)
    .bind(&repository.html_url)
    .bind(&repository.default_branch)
    .bind(repository.private)
    .bind(repository.archived)
    .bind(repository.fork)
    .bind(&repository.description)
    .bind(repository.pushed_at)
    .bind(Json(&repository.original))
    .bind(account)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE projects.projects p SET name = $2 \
         WHERE p.name <> $2 AND p.id = (SELECT r.project_id FROM projects.repositories r WHERE r.id = $1) \
         AND $1 = (SELECT f.id FROM projects.repositories f WHERE f.project_id = p.id \
                   ORDER BY f.first_seen_at, f.id LIMIT 1)",
    )
    .bind(id)
    .bind(name)
    .execute(conn)
    .await
    .map(drop)
}

/// Clears `gone` of a left-out repository the listing holds again; nothing else is written.
pub(super) async fn returned(
    conn: &mut PgConnection,
    id: RepositoryId,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects.repositories SET gone_at = NULL, updated_at = $2 WHERE id = $1")
        .bind(id)
        .bind(now)
        .execute(conn)
        .await
        .map(drop)
}

/// After a complete listing by `account`: marks gone each of its repositories the listing does
/// not hold, and stamps when the others were listed. Returns how many it marked gone.
pub(super) async fn listed(
    conn: &mut PgConnection,
    account: SourceAccountId,
    external_ids: &[String],
    now: DateTime<Utc>,
) -> Result<u64, sqlx::Error> {
    let gone = sqlx::query(
        "UPDATE projects.repositories SET gone_at = $3, updated_at = $3 \
         WHERE account_id = $1 AND forge = $4 AND gone_at IS NULL AND NOT (external_id = ANY($2))",
    )
    .bind(account)
    .bind(external_ids)
    .bind(now)
    .bind(FORGE)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE projects.repositories SET listed_at = $2 WHERE forge = $3 AND external_id = ANY($1)",
    )
    .bind(external_ids)
    .bind(now)
    .bind(FORGE)
    .execute(conn)
    .await?;
    Ok(gone.rows_affected())
}

/// The GitHub ids of every left-out repository (PRO-002).
pub(super) async fn left_out_ids(conn: &mut PgConnection) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT external_id FROM projects.repositories WHERE forge = $1 AND left_out_at IS NOT NULL",
    )
    .bind(FORGE)
    .fetch_all(conn)
    .await
}

/// Deletes the repositories of `account`, then each of their projects that has none left. The
/// operator's decisions outlive the account (PRO-002, PRO-004): a left-out repository, and one
/// whose project holds a confirmed resource, is kept with no account, so the account that
/// lists it next adopts it as it is. Returns how many repositories it deleted.
pub(super) async fn forget(
    conn: &mut PgConnection,
    account: SourceAccountId,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE projects.repositories r SET account_id = NULL \
         WHERE r.account_id = $1 AND (r.left_out_at IS NOT NULL OR EXISTS ( \
             SELECT 1 FROM projects.resources c \
             WHERE c.project_id = r.project_id AND c.state = 'confirmed'))",
    )
    .bind(account)
    .execute(&mut *conn)
    .await?;
    let projects: Vec<ProjectId> = sqlx::query_scalar(
        "DELETE FROM projects.repositories WHERE account_id = $1 RETURNING project_id",
    )
    .bind(account)
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query(
        "DELETE FROM projects.projects p WHERE p.id = ANY($1) \
         AND NOT EXISTS (SELECT 1 FROM projects.repositories r WHERE r.project_id = p.id)",
    )
    .bind(&projects)
    .execute(conn)
    .await?;
    Ok(u64::try_from(projects.len()).unwrap_or(u64::MAX))
}
