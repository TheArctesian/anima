//! All SQL on schema `files`: I/O only.

mod proposals;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::{
    location::Position,
    model::{Archive, Attachment},
};
use crate::{
    engine::records::Reference,
    ids::{ArchiveId, AttachmentId},
};

pub(super) use proposals::{decide, insert_proposal, proposal, proposals};

const ATTACHMENT: &str = "a.id, a.hash, a.owner, a.name, a.media_type, a.archive_id, \
    a.latitude, a.longitude, a.created_at, p.key_name, p.size";

/// The key of a piece row: `<key name>:<hash>`.
pub(super) fn piece_key(key_name: &str, hash: &str) -> String {
    format!("{key_name}:{hash}")
}

fn unreadable(column: &str) -> sqlx::Error {
    sqlx::Error::Decode(format!("bad files {column}").into())
}

#[derive(sqlx::FromRow)]
struct AttachmentRow {
    id: AttachmentId,
    hash: String,
    owner: String,
    name: String,
    media_type: String,
    archive_id: Option<ArchiveId>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    created_at: DateTime<Utc>,
    key_name: String,
    size: i64,
}

impl TryFrom<AttachmentRow> for Attachment {
    type Error = sqlx::Error;

    fn try_from(row: AttachmentRow) -> Result<Self, Self::Error> {
        let (_, hash) = row.hash.split_once(':').ok_or_else(|| unreadable("hash"))?;
        Ok(Self {
            id: row.id,
            owner: Reference::parse(&row.owner).map_err(|_| unreadable("owner"))?,
            name: row.name,
            media_type: row.media_type,
            hash: hash.to_owned(),
            key_name: row.key_name,
            size: u64::try_from(row.size).map_err(|_| unreadable("size"))?,
            archive: row.archive_id,
            position: row
                .latitude
                .zip(row.longitude)
                .map(|(latitude, longitude)| Position {
                    latitude,
                    longitude,
                }),
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ArchiveRow {
    id: ArchiveId,
    owner: String,
    name: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<ArchiveRow> for Archive {
    type Error = sqlx::Error;

    fn try_from(row: ArchiveRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            owner: Reference::parse(&row.owner).map_err(|_| unreadable("owner"))?,
            name: row.name,
            created_at: row.created_at,
        })
    }
}

/// Holds the piece until the transaction ends, so that an upload and a delete of the same
/// piece take turns.
pub(super) async fn lock_piece(conn: &mut PgConnection, piece: &str) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 2050))")
        .bind(piece)
        .execute(conn)
        .await
        .map(drop)
}

pub(super) async fn piece_exists(
    conn: &mut PgConnection,
    piece: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM files.piece WHERE hash = $1)")
        .bind(piece)
        .fetch_one(conn)
        .await
}

/// Stores the row of a piece; a row that exists stays.
pub(super) async fn insert_piece(
    conn: &mut PgConnection,
    (key_name, hash): (&str, &str),
    (size, sealed_size): (u64, u64),
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let size = i64::try_from(size).map_err(|_| unreadable("size"))?;
    let sealed_size = i64::try_from(sealed_size).map_err(|_| unreadable("sealed size"))?;
    sqlx::query(
        "INSERT INTO files.piece (hash, key_name, size, sealed_size, stored_at) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (hash) DO NOTHING",
    )
    .bind(piece_key(key_name, hash))
    .bind(key_name)
    .bind(size)
    .bind(sealed_size)
    .bind(at)
    .execute(conn)
    .await
    .map(drop)
}

pub(super) async fn insert_attachment(
    conn: &mut PgConnection,
    attachment: &Attachment,
    by: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO files.attachment (id, hash, owner, name, media_type, archive_id, latitude, \
         longitude, created_at, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(attachment.id)
    .bind(piece_key(&attachment.key_name, &attachment.hash))
    .bind(attachment.owner.as_str())
    .bind(&attachment.name)
    .bind(&attachment.media_type)
    .bind(attachment.archive)
    .bind(attachment.position.map(|p| p.latitude))
    .bind(attachment.position.map(|p| p.longitude))
    .bind(attachment.created_at)
    .bind(by)
    .execute(conn)
    .await
    .map(drop)
}

/// The attachments of `owner`, newest first.
pub(super) async fn attachments(
    db: impl PgExecutor<'_>,
    owner: &Reference,
    limit: i64,
) -> Result<Vec<Attachment>, sqlx::Error> {
    let rows: Vec<AttachmentRow> = sqlx::query_as(&format!(
        "SELECT {ATTACHMENT} FROM files.attachment a JOIN files.piece p ON p.hash = a.hash \
         WHERE a.owner = $1 ORDER BY a.created_at DESC, a.id DESC LIMIT $2"
    ))
    .bind(owner.as_str())
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(Attachment::try_from).collect()
}

pub(super) async fn attachment(
    db: impl PgExecutor<'_>,
    id: AttachmentId,
) -> Result<Option<Attachment>, sqlx::Error> {
    let row: Option<AttachmentRow> = sqlx::query_as(&format!(
        "SELECT {ATTACHMENT} FROM files.attachment a JOIN files.piece p ON p.hash = a.hash \
         WHERE a.id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(Attachment::try_from).transpose()
}

/// Deletes the attachments `WHERE <filter> = $1` and returns them.
async fn delete_attachments_where(
    conn: &mut PgConnection,
    filter: &str,
    id: Uuid,
) -> Result<Vec<Attachment>, sqlx::Error> {
    let rows: Vec<AttachmentRow> = sqlx::query_as(&format!(
        "WITH gone AS (DELETE FROM files.attachment WHERE {filter} = $1 RETURNING *) \
         SELECT {ATTACHMENT} FROM gone a JOIN files.piece p ON p.hash = a.hash"
    ))
    .bind(id)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(Attachment::try_from).collect()
}

pub(super) async fn delete_attachment(
    conn: &mut PgConnection,
    id: AttachmentId,
) -> Result<Option<Attachment>, sqlx::Error> {
    Ok(delete_attachments_where(conn, "id", id.into()).await?.pop())
}

pub(super) async fn delete_archive_attachments(
    conn: &mut PgConnection,
    archive: ArchiveId,
) -> Result<Vec<Attachment>, sqlx::Error> {
    delete_attachments_where(conn, "archive_id", archive.into()).await
}

/// Deletes the rows of the named pieces that no attachment uses and gives their key and hash.
pub(super) async fn delete_unused_pieces(
    conn: &mut PgConnection,
    pieces: &[String],
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let gone: Vec<(String, String)> = sqlx::query_as(
        "DELETE FROM files.piece p WHERE p.hash = ANY($1) AND NOT EXISTS \
         (SELECT 1 FROM files.attachment a WHERE a.hash = p.hash) RETURNING p.key_name, p.hash",
    )
    .bind(pieces)
    .fetch_all(conn)
    .await?;
    gone.into_iter()
        .map(|(key, piece)| match piece.split_once(':') {
            Some((_, hash)) => Ok((key, hash.to_owned())),
            None => Err(unreadable("hash")),
        })
        .collect()
}

pub(super) async fn insert_archive(
    conn: &mut PgConnection,
    archive: &Archive,
    by: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO files.archive (id, owner, name, created_at, created_by) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(archive.id)
    .bind(archive.owner.as_str())
    .bind(&archive.name)
    .bind(archive.created_at)
    .bind(by)
    .execute(conn)
    .await
    .map(drop)
}

pub(super) async fn archive(
    db: impl PgExecutor<'_>,
    id: ArchiveId,
) -> Result<Option<Archive>, sqlx::Error> {
    let row: Option<ArchiveRow> =
        sqlx::query_as("SELECT id, owner, name, created_at FROM files.archive WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?;
    row.map(Archive::try_from).transpose()
}

pub(super) async fn delete_archive(
    conn: &mut PgConnection,
    id: ArchiveId,
) -> Result<Option<Archive>, sqlx::Error> {
    let row: Option<ArchiveRow> = sqlx::query_as(
        "DELETE FROM files.archive WHERE id = $1 RETURNING id, owner, name, created_at",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    row.map(Archive::try_from).transpose()
}
