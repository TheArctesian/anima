//! The signs of a repository and the resources of a project (PRO-003, PRO-004): I/O only.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    ids::{ProjectId, RepositoryId, ResourceId},
    work::projects::model::{Held, ResourceKind, ResourceState, Sign},
};

/// One stored sign: what it is and where (`None` for a deployment record).
pub(in crate::work::projects) type Found = (Sign, Option<String>);

/// The signs of `repository` from `origin`, in a fixed order. A row with a sign this version
/// does not know is skipped.
pub(in crate::work::projects) async fn of_origin(
    conn: &mut PgConnection,
    repository: RepositoryId,
    origin: &str,
) -> Result<Vec<Found>, sqlx::Error> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT sign, path FROM projects.signs WHERE repository_id = $1 AND origin = $2 \
         ORDER BY sign, path",
    )
    .bind(repository)
    .bind(origin)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(sign, path)| Sign::parse(&sign).map(|sign| (sign, path)))
        .collect())
}

/// Replaces the signs of `repository` from `origin` with `signs`, which hold no repeat.
pub(in crate::work::projects) async fn replace(
    conn: &mut PgConnection,
    (repository, origin): (RepositoryId, &str),
    signs: &[Found],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM projects.signs WHERE repository_id = $1 AND origin = $2")
        .bind(repository)
        .bind(origin)
        .execute(&mut *conn)
        .await?;
    let kinds: Vec<&str> = signs.iter().map(|(sign, _)| sign.as_str()).collect();
    let paths: Vec<Option<&str>> = signs.iter().map(|(_, path)| path.as_deref()).collect();
    sqlx::query(
        "INSERT INTO projects.signs (repository_id, origin, sign, path) \
         SELECT $1, $2, sign, path FROM UNNEST($3::text[], $4::text[]) AS s (sign, path)",
    )
    .bind(repository)
    .bind(origin)
    .bind(kinds)
    .bind(paths)
    .execute(conn)
    .await
    .map(drop)
}

/// The kinds of sign that the repositories of `project` that are not left out show.
pub(in crate::work::projects) async fn of_project(
    conn: &mut PgConnection,
    project: ProjectId,
) -> Result<Vec<Sign>, sqlx::Error> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT s.sign FROM projects.signs s \
         JOIN projects.repositories r ON r.id = s.repository_id \
         WHERE r.project_id = $1 AND r.left_out_at IS NULL",
    )
    .bind(project)
    .fetch_all(conn)
    .await?;
    Ok(rows.iter().filter_map(|sign| Sign::parse(sign)).collect())
}

/// The resources of `project`, locked until the transaction ends.
pub(in crate::work::projects) async fn held(
    conn: &mut PgConnection,
    project: ProjectId,
) -> Result<Vec<Held>, sqlx::Error> {
    let rows: Vec<(ResourceId, String, String)> = sqlx::query_as(
        "SELECT id, kind, state FROM projects.resources WHERE project_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(project)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, kind, state)| {
            Some(Held {
                id,
                kind: ResourceKind::parse(&kind)?,
                state: ResourceState::parse(&state)?,
            })
        })
        .collect())
}

/// Proposes `kinds` for `project` as found in the repository.
pub(in crate::work::projects) async fn propose(
    conn: &mut PgConnection,
    project: ProjectId,
    kinds: &[ResourceKind],
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    for kind in kinds {
        sqlx::query(
            "INSERT INTO projects.resources (id, project_id, kind, state, found_at) \
             VALUES ($1, $2, $3, 'found', $4) ON CONFLICT (project_id, kind) DO NOTHING",
        )
        .bind(ResourceId::from(Uuid::now_v7()))
        .bind(project)
        .bind(kind.as_str())
        .bind(now)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Removes the found resources among `ids`; a confirmed one stays.
pub(in crate::work::projects) async fn withdraw(
    conn: &mut PgConnection,
    ids: &[ResourceId],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM projects.resources WHERE id = ANY($1) AND state = 'found'")
        .bind(ids)
        .execute(conn)
        .await
        .map(drop)
}
