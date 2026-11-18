//! The only SQL for `events.audit_chain` (LOG-021 to LOG-027).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::fleet::events::chain::{Hash, Link, StoredEntry, Walk};

/// The columns of an audit entry that the hash covers, in the order of [`EntryRow`].
pub(in crate::fleet::events) const ENTRY_COLUMNS: &str = "id, at, operator_id, operator_name, \
     action, target_type, target_id, client_addr, change, actor_kind, token_kind, api_key_name";

/// The links that one statement of [`link_existing`] reads and writes.
const PAGE: i64 = 1000;

/// A stored `audit_log` row with each column the hash covers.
#[derive(sqlx::FromRow)]
pub(in crate::fleet::events) struct EntryRow {
    id: Uuid,
    at: DateTime<Utc>,
    operator_id: Uuid,
    operator_name: String,
    action: String,
    target_type: String,
    target_id: String,
    client_addr: IpAddr,
    change: Value,
    actor_kind: Option<String>,
    token_kind: Option<String>,
    api_key_name: Option<String>,
}

impl From<EntryRow> for StoredEntry {
    fn from(row: EntryRow) -> Self {
        Self {
            id: row.id,
            at: row.at,
            operator_id: row.operator_id,
            operator_name: row.operator_name,
            action: row.action,
            target_type: row.target_type,
            target_id: row.target_id,
            client: row.client_addr,
            change: row.change,
            actor_kind: row.actor_kind,
            token_kind: row.token_kind,
            api_key_name: row.api_key_name,
        }
    }
}

/// A link and, through a left join, the columns of its entry: null when the entry is gone.
#[derive(sqlx::FromRow)]
struct LinkRow {
    seq: i64,
    entry_id: Uuid,
    entry_at: DateTime<Utc>,
    previous: Vec<u8>,
    hash: Vec<u8>,
    id: Option<Uuid>,
    at: Option<DateTime<Utc>>,
    operator_id: Option<Uuid>,
    operator_name: Option<String>,
    action: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
    client_addr: Option<IpAddr>,
    change: Option<Value>,
    actor_kind: Option<String>,
    token_kind: Option<String>,
    api_key_name: Option<String>,
}

impl TryFrom<LinkRow> for (Link, Option<StoredEntry>) {
    type Error = sqlx::Error;

    fn try_from(row: LinkRow) -> Result<Self, Self::Error> {
        let link = Link {
            seq: row.seq,
            entry_id: row.entry_id,
            entry_at: row.entry_at,
            previous: hash_of(row.previous)?,
            hash: hash_of(row.hash)?,
        };
        let entry = row.id.map(|id| {
            Some(StoredEntry {
                id,
                at: row.at?,
                operator_id: row.operator_id?,
                operator_name: row.operator_name?,
                action: row.action?,
                target_type: row.target_type?,
                target_id: row.target_id?,
                client: row.client_addr?,
                change: row.change?,
                actor_kind: row.actor_kind,
                token_kind: row.token_kind,
                api_key_name: row.api_key_name,
            })
        });
        let entry = entry
            .map(|entry| entry.ok_or_else(|| sqlx::Error::Decode("a null audit column".into())))
            .transpose()?;
        Ok((link, entry))
    }
}

fn hash_of(bytes: Vec<u8>) -> Result<Hash, sqlx::Error> {
    Hash::try_from(bytes).map_err(|_| sqlx::Error::Decode("a link hash is not 32 bytes".into()))
}

/// Takes the lock of the chain until the end of the caller's transaction (LOG-024).
pub(in crate::fleet::events) async fn lock(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("select pg_advisory_xact_lock(hashtextextended('events.audit_chain', 0))")
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The sequence number and hash of the newest link, or `None` for an empty chain.
pub(in crate::fleet::events) async fn head(
    conn: &mut PgConnection,
) -> Result<Option<(i64, Hash)>, sqlx::Error> {
    let row: Option<(i64, Vec<u8>)> =
        sqlx::query_as("select seq, hash from events.audit_chain order by seq desc limit 1")
            .fetch_optional(&mut *conn)
            .await?;
    row.map(|(seq, hash)| Ok((seq, hash_of(hash)?))).transpose()
}

/// Inserts `links` in one statement.
pub(in crate::fleet::events) async fn insert_links(
    conn: &mut PgConnection,
    links: &[Link],
) -> Result<(), sqlx::Error> {
    let seqs: Vec<i64> = links.iter().map(|link| link.seq).collect();
    let ids: Vec<Uuid> = links.iter().map(|link| link.entry_id).collect();
    let ats: Vec<DateTime<Utc>> = links.iter().map(|link| link.entry_at).collect();
    let previous: Vec<Vec<u8>> = links.iter().map(|link| link.previous.to_vec()).collect();
    let hashes: Vec<Vec<u8>> = links.iter().map(|link| link.hash.to_vec()).collect();
    sqlx::query(
        "insert into events.audit_chain (seq, entry_id, entry_at, previous, hash) \
         select * from unnest($1::pg_catalog.int8[], $2::pg_catalog.uuid[], \
         $3::pg_catalog.timestamptz[], $4::pg_catalog.bytea[], $5::pg_catalog.bytea[])",
    )
    .bind(seqs)
    .bind(ids)
    .bind(ats)
    .bind(previous)
    .bind(hashes)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Up to [`PAGE`] audit entries after `after` (a time and an ID), oldest first.
pub(in crate::fleet::events) async fn entries_after(
    conn: &mut PgConnection,
    after: Option<(DateTime<Utc>, Uuid)>,
) -> Result<Vec<StoredEntry>, sqlx::Error> {
    let rows: Vec<EntryRow> = sqlx::query_as(&format!(
        "select {ENTRY_COLUMNS} from events.audit_log \
         where $1::pg_catalog.timestamptz is null or (at, id) > ($1, $2) \
         order by at, id limit $3"
    ))
    .bind(after.map(|(at, _)| at))
    .bind(after.map(|(_, id)| id))
    .bind(PAGE)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(StoredEntry::from).collect())
}

/// Feeds each link and its entry to `walk`, in the order of the sequence number, until the
/// walk stops; then returns the oldest entry that has no link. One read-only transaction at
/// the level REPEATABLE READ, so both reads see the same entries and links.
pub(in crate::fleet::events) async fn walk(
    db: &PgPool,
    walk: &mut Walk,
) -> Result<Option<(Uuid, DateTime<Utc>)>, sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    {
        let columns = ENTRY_COLUMNS
            .split(", ")
            .map(|column| format!("a.{column}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "select c.seq, c.entry_id, c.entry_at, c.previous, c.hash, {columns} \
             from events.audit_chain c left join events.audit_log a on a.id = c.entry_id \
             order by c.seq"
        );
        let mut rows = sqlx::query_as::<_, LinkRow>(&sql).fetch(&mut *tx);
        while let Some(row) = rows.try_next().await? {
            let (link, entry) = <(Link, Option<StoredEntry>)>::try_from(row)?;
            if !walk.step(&link, entry.as_ref()) {
                break;
            }
        }
    }
    let unlinked = sqlx::query_as(
        "select a.id, a.at from events.audit_log a \
         where not exists (select from events.audit_chain c where c.entry_id = a.id) \
         order by a.at, a.id limit 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(unlinked)
}
