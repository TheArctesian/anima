//! The rows of `sealed_records` (K-E5 Section 3). Each query names its columns.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::{Error, NewRecord, Record};
use crate::{
    engine::records::Reference,
    ids::{KeyOfferId, SealedRecordId},
};

const RECORD: &str = "id, owner, extension, sealed, sealed_index, key_id, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct Row {
    id: SealedRecordId,
    owner: String,
    extension: String,
    sealed: Vec<u8>,
    sealed_index: Option<Vec<u8>>,
    key_id: Vec<u8>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Record {
    type Error = Error;

    fn try_from(row: Row) -> Result<Self, Error> {
        Ok(Self {
            id: row.id,
            owner: Reference::parse(&row.owner).map_err(|_| Error::Stored)?,
            extension: row.extension,
            sealed: row.sealed,
            sealed_index: row.sealed_index,
            key_id: row.key_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub(super) async fn read(
    conn: &mut PgConnection,
    owner: &Reference,
) -> Result<Option<Record>, Error> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {RECORD} FROM sealed_records.record WHERE owner = $1"
    ))
    .bind(owner.as_str())
    .fetch_optional(conn)
    .await?;
    row.map(TryInto::try_into).transpose()
}

/// Inserts the record of the owner or replaces its bytes. `true` for a record that is new: the
/// row is locked first, so the answer is of this transaction.
pub(super) async fn upsert(
    conn: &mut PgConnection,
    id: SealedRecordId,
    new: &NewRecord,
    now: DateTime<Utc>,
) -> Result<(Record, bool), Error> {
    let had: Option<SealedRecordId> =
        sqlx::query_scalar("SELECT id FROM sealed_records.record WHERE owner = $1 FOR UPDATE")
            .bind(new.owner.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    let row: Row = sqlx::query_as(&format!(
        "INSERT INTO sealed_records.record \
         (id, owner, extension, sealed, sealed_index, key_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7) \
         ON CONFLICT (owner) DO UPDATE SET extension = excluded.extension, \
         sealed = excluded.sealed, sealed_index = excluded.sealed_index, \
         key_id = excluded.key_id, updated_at = excluded.updated_at \
         RETURNING {RECORD}"
    ))
    .bind(id)
    .bind(new.owner.as_str())
    .bind(&new.extension)
    .bind(&new.sealed)
    .bind(new.sealed_index.as_deref())
    .bind(&new.key_id)
    .bind(now)
    .fetch_one(conn)
    .await?;
    Ok((row.try_into()?, had.is_none()))
}

pub(super) async fn delete(
    conn: &mut PgConnection,
    owner: &Reference,
) -> Result<Option<Record>, Error> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "DELETE FROM sealed_records.record WHERE owner = $1 RETURNING {RECORD}"
    ))
    .bind(owner.as_str())
    .fetch_optional(conn)
    .await?;
    row.map(TryInto::try_into).transpose()
}

pub(super) async fn insert_offer(
    conn: &mut PgConnection,
    id: KeyOfferId,
    sealed: &[u8],
    (now, expires_at): (DateTime<Utc>, DateTime<Utc>),
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sealed_records.key_offer WHERE expires_at <= $1")
        .bind(now)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO sealed_records.key_offer (id, sealed, created_at, expires_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(sealed)
    .bind(now)
    .bind(expires_at)
    .execute(conn)
    .await
    .map(drop)
}

/// Deletes the offer and gives its bytes when it did not end before `now`.
pub(super) async fn take_offer(
    conn: &mut PgConnection,
    id: KeyOfferId,
    now: DateTime<Utc>,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let taken: Option<(Vec<u8>, DateTime<Utc>)> = sqlx::query_as(
        "DELETE FROM sealed_records.key_offer WHERE id = $1 RETURNING sealed, expires_at",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?;
    Ok(taken
        .filter(|(_, expires_at)| *expires_at > now)
        .map(|(sealed, _)| sealed))
}
