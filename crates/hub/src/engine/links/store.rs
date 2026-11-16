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
