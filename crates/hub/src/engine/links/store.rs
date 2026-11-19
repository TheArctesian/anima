//! All SQL on schema `links`: I/O only.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::model::{Link, NewLink, Relation};
use crate::{engine::records::Reference, ids::RecordLinkId};

const COLUMNS: &str = "id, source, target, kind, label, created_at";

#[derive(sqlx::FromRow)]
struct LinkRow {
    id: RecordLinkId,
    source: String,
    target: String,
    kind: String,
    label: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<LinkRow> for Link {
    type Error = sqlx::Error;

    fn try_from(row: LinkRow) -> Result<Self, Self::Error> {
        let unreadable = |column: &str| sqlx::Error::Decode(format!("bad link {column}").into());
        Ok(Self {
            id: row.id,
            source: Reference::parse(&row.source).map_err(|_| unreadable("source"))?,
            target: Reference::parse(&row.target).map_err(|_| unreadable("target"))?,
            relation: Relation::parse(&row.kind).ok_or_else(|| unreadable("kind"))?,
            label: row.label,
            created_at: row.created_at,
        })
    }
}

/// Inserts the link; `None` when a link with the same source, target and kind exists.
pub(super) async fn insert(
    conn: &mut PgConnection,
    new: &NewLink,
    (at, by): (DateTime<Utc>, &str),
) -> Result<Option<Link>, sqlx::Error> {
    let row: Option<LinkRow> = sqlx::query_as(&format!(
        "INSERT INTO links.record_link (id, source, target, kind, label, created_at, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (source, target, kind) DO NOTHING RETURNING {COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(new.source.as_str())
    .bind(new.target.as_str())
    .bind(new.relation.as_str())
    .bind(new.label.as_deref())
    .bind(at)
    .bind(by)
    .fetch_optional(conn)
    .await?;
    row.map(Link::try_from).transpose()
}

/// The stored link with the source, target and kind of `new`, locked until the transaction
/// ends.
pub(super) async fn existing(
    conn: &mut PgConnection,
    new: &NewLink,
) -> Result<Option<Link>, sqlx::Error> {
    let row: Option<LinkRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM links.record_link \
         WHERE source = $1 AND target = $2 AND kind = $3 FOR UPDATE"
    ))
    .bind(new.source.as_str())
    .bind(new.target.as_str())
    .bind(new.relation.as_str())
    .fetch_optional(conn)
    .await?;
    row.map(Link::try_from).transpose()
}

/// Writes the label of one link.
pub(super) async fn set_label(
    conn: &mut PgConnection,
    id: RecordLinkId,
    label: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE links.record_link SET label = $2 WHERE id = $1")
        .bind(id)
        .bind(label)
        .execute(conn)
        .await
        .map(drop)
}

/// Up to `limit` links that start or end at `record`, newest first.
pub(super) async fn touching(
    db: impl PgExecutor<'_>,
    record: &Reference,
    limit: i64,
) -> Result<Vec<Link>, sqlx::Error> {
    let rows: Vec<LinkRow> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM links.record_link WHERE source = $1 OR target = $1 \
         ORDER BY created_at DESC, id DESC LIMIT $2"
    ))
    .bind(record.as_str())
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Link::try_from).collect()
}

/// Deletes one link and returns it; `None` when there is no such link.
pub(super) async fn delete(
    conn: &mut PgConnection,
    id: RecordLinkId,
) -> Result<Option<Link>, sqlx::Error> {
    let row: Option<LinkRow> = sqlx::query_as(&format!(
        "DELETE FROM links.record_link WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(Link::try_from).transpose()
}

/// How many links start or end at `record`.
pub(super) async fn count(conn: &mut PgConnection, record: &Reference) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM links.record_link WHERE source = $1 OR target = $1")
        .bind(record.as_str())
        .fetch_one(conn)
        .await
}

/// Moves the links of `from` to `to` (all of them, or only those in `only`). Links that would
/// become a link to itself, or a second copy of one that `to` has, are deleted first. Returns
/// how many links moved.
pub(super) async fn retarget(
    conn: &mut PgConnection,
    (from, to): (&Reference, &Reference),
    only: Option<&[Uuid]>,
) -> Result<u64, sqlx::Error> {
    const FILTER: &str = "($3::uuid[] IS NULL OR r.id = ANY($3))";
    let deletions = [
        format!(
            "DELETE FROM links.record_link r WHERE {FILTER} AND \
             ((r.source = $1 AND r.target = $2) OR (r.source = $2 AND r.target = $1))"
        ),
        format!(
            "DELETE FROM links.record_link r WHERE {FILTER} AND r.source = $1 AND EXISTS \
             (SELECT 1 FROM links.record_link o \
              WHERE o.source = $2 AND o.target = r.target AND o.kind = r.kind)"
        ),
        format!(
            "DELETE FROM links.record_link r WHERE {FILTER} AND r.target = $1 AND EXISTS \
             (SELECT 1 FROM links.record_link o \
              WHERE o.target = $2 AND o.source = r.source AND o.kind = r.kind)"
        ),
    ];
    for statement in deletions {
        sqlx::query(&statement)
            .bind(from.as_str())
            .bind(to.as_str())
            .bind(only)
            .execute(&mut *conn)
            .await?;
    }
    let mut moved = 0;
    for side in ["source", "target"] {
        let result = sqlx::query(&format!(
            "UPDATE links.record_link r SET {side} = $2 WHERE {FILTER} AND r.{side} = $1"
        ))
        .bind(from.as_str())
        .bind(to.as_str())
        .bind(only)
        .execute(&mut *conn)
        .await?;
        moved += result.rows_affected();
    }
    Ok(moved)
}
