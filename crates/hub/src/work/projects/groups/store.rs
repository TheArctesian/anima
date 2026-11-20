//! The only SQL of the table of groups and of the group of a project (PRO-059 to PRO-063).
//! I/O only.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use crate::ids::{ProjectGroupId, ProjectId};

/// A group with the number of its projects that have a repository that is not left out
/// (PRO-064).
#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct GroupRow {
    pub id: ProjectGroupId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub project_count: i64,
}

/// A project whose group a change moved: its slug and the group that it had before.
#[derive(Clone, Debug, sqlx::FromRow)]
pub(in crate::work::projects) struct MovedRow {
    pub id: ProjectId,
    pub slug: String,
    pub before: Option<ProjectGroupId>,
}

const GROUP: &str = "SELECT g.id, g.name, g.created_at, \
    (SELECT count(*) FROM projects.projects p WHERE p.group_id = g.id AND EXISTS ( \
        SELECT 1 FROM projects.repositories r WHERE r.project_id = p.id AND r.left_out_at IS NULL \
    )) AS project_count FROM projects.groups g";

/// Each group, in name order.
pub(in crate::work::projects) async fn all(db: &PgPool) -> Result<Vec<GroupRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "{GROUP} ORDER BY lower(g.name) COLLATE \"C\", g.id"
    ))
    .fetch_all(db)
    .await
}

/// The group of each of `projects` that has one: `(project, group)`.
pub(in crate::work::projects) async fn of_projects(
    db: &PgPool,
    projects: &[ProjectId],
) -> Result<Vec<(ProjectId, GroupRow)>, sqlx::Error> {
    let pairs: Vec<(ProjectId, ProjectGroupId)> = sqlx::query_as(
        "SELECT id, group_id FROM projects.projects WHERE id = ANY($1) AND group_id IS NOT NULL",
    )
    .bind(projects)
    .fetch_all(db)
    .await?;
    let mut ids: Vec<ProjectGroupId> = Vec::new();
    for (_, group) in &pairs {
        if !ids.contains(group) {
            ids.push(*group);
        }
    }
    let groups: Vec<GroupRow> = sqlx::query_as(&format!("{GROUP} WHERE g.id = ANY($1)"))
        .bind(&ids)
        .fetch_all(db)
        .await?;
    Ok(pairs
        .into_iter()
        .filter_map(|(project, group)| {
            let row = groups.iter().find(|row| row.id == group)?;
            Some((project, row.clone()))
        })
        .collect())
}

/// The group `id`.
pub(in crate::work::projects) async fn one(
    conn: &mut PgConnection,
    id: ProjectGroupId,
) -> Result<Option<GroupRow>, sqlx::Error> {
    sqlx::query_as(&format!("{GROUP} WHERE g.id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await
}

/// Locks the group `id` for the change of this transaction; false when there is none.
pub(in crate::work::projects) async fn lock(
    conn: &mut PgConnection,
    id: ProjectGroupId,
) -> Result<bool, sqlx::Error> {
    let found: Option<ProjectGroupId> =
        sqlx::query_scalar("SELECT id FROM projects.groups WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(conn)
            .await?;
    Ok(found.is_some())
}

/// Whether a group other than `except` has this name, with the case of the letters ignored.
pub(in crate::work::projects) async fn name_taken(
    conn: &mut PgConnection,
    name: &str,
    except: Option<ProjectGroupId>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM projects.groups WHERE lower(name) = lower($1) \
         AND ($2::uuid IS NULL OR id <> $2))",
    )
    .bind(name)
    .bind(except)
    .fetch_one(conn)
    .await
}

pub(in crate::work::projects) async fn insert(
    conn: &mut PgConnection,
    (id, name): (ProjectGroupId, &str),
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO projects.groups (id, name, created_at) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(now)
        .execute(conn)
        .await
        .map(drop)
}

pub(in crate::work::projects) async fn rename(
    conn: &mut PgConnection,
    id: ProjectGroupId,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE projects.groups SET name = $2 WHERE id = $1")
        .bind(id)
        .bind(name)
        .execute(conn)
        .await
        .map(drop)
}

/// Deletes the group `id`; its projects keep each repository and get no group (PRO-062).
pub(in crate::work::projects) async fn delete(
    conn: &mut PgConnection,
    id: ProjectGroupId,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM projects.groups WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await
        .map(drop)
}

/// The ids of `projects` that exist.
pub(in crate::work::projects) async fn existing(
    conn: &mut PgConnection,
    projects: &[ProjectId],
) -> Result<Vec<ProjectId>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM projects.projects WHERE id = ANY($1)")
        .bind(projects)
        .fetch_all(conn)
        .await
}

/// Puts each of `projects` in `group`, or in no group; returns each project that moved.
pub(in crate::work::projects) async fn set_group(
    conn: &mut PgConnection,
    projects: &[ProjectId],
    group: Option<ProjectGroupId>,
) -> Result<Vec<MovedRow>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE projects.projects p SET group_id = $2 FROM projects.projects old \
         WHERE p.id = old.id AND p.id = ANY($1) AND p.group_id IS DISTINCT FROM $2 \
         RETURNING p.id, p.slug, old.group_id AS before",
    )
    .bind(projects)
    .bind(group)
    .fetch_all(conn)
    .await
}
