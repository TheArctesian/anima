//! All SQL on schema `locations`: I/O only.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use super::{
    context::{Actor, End, Stored as StoredContext},
    model::{IngestToken, Origin},
};
use crate::ids::{EventId, IngestTokenId, SampleId};

/// One row of `locations.sample`, still sealed.
#[derive(sqlx::FromRow)]
pub(super) struct SampleRow {
    pub id: SampleId,
    pub provider: String,
    pub account: String,
    pub sample_key: String,
    pub observed_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub sealed: Vec<u8>,
}

const SAMPLE: &str = "id, provider, account, sample_key, observed_at, received_at, sealed";

/// Stores one sample; `false` when the hub already has a sample of this provider, account
/// and key, which then stays as it is.
pub(super) async fn insert_sample(
    conn: &mut PgConnection,
    (id, origin, key): (SampleId, &Origin, &str),
    (observed_at, received_at): (Option<DateTime<Utc>>, DateTime<Utc>),
    sealed: &[u8],
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query(
        "INSERT INTO locations.sample \
         (id, provider, account, sample_key, observed_at, received_at, sealed) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (provider, account, sample_key) DO NOTHING",
    )
    .bind(id)
    .bind(origin.provider.as_str())
    .bind(&origin.account)
    .bind(key)
    .bind(observed_at)
    .bind(received_at)
    .bind(sealed)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// The samples observed from `from` to `to`, oldest first, at most `limit`.
pub(super) async fn samples_between(
    db: impl PgExecutor<'_>,
    (from, to): (DateTime<Utc>, DateTime<Utc>),
    limit: i64,
) -> Result<Vec<SampleRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {SAMPLE} FROM locations.sample \
         WHERE observed_at >= $1 AND observed_at <= $2 \
         ORDER BY observed_at, id LIMIT $3"
    ))
    .bind(from)
    .bind(to)
    .bind(limit)
    .fetch_all(db)
    .await
}

/// The samples that the hub got after `after` and not after `until`, in the order of their
/// arrival, at most `limit`.
pub(super) async fn samples_received(
    db: impl PgExecutor<'_>,
    (after, until): (DateTime<Utc>, DateTime<Utc>),
    limit: i64,
) -> Result<Vec<SampleRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {SAMPLE} FROM locations.sample \
         WHERE received_at > $1 AND received_at <= $2 \
         ORDER BY received_at, id LIMIT $3"
    ))
    .bind(after)
    .bind(until)
    .bind(limit)
    .fetch_all(db)
    .await
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    id: IngestTokenId,
    name: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl From<TokenRow> for IngestToken {
    fn from(row: TokenRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            revoked_at: row.revoked_at,
        }
    }
}

const TOKEN: &str = "id, name, created_at, last_used_at, revoked_at";

pub(super) async fn insert_token(
    conn: &mut PgConnection,
    token: &IngestToken,
    hash: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO locations.ingest_token (id, name, token_hash, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(token.id)
    .bind(&token.name)
    .bind(hash)
    .bind(token.created_at)
    .execute(conn)
    .await
    .map(drop)
}

/// Every ingest token, newest first.
pub(super) async fn tokens(db: impl PgExecutor<'_>) -> Result<Vec<IngestToken>, sqlx::Error> {
    let rows: Vec<TokenRow> = sqlx::query_as(&format!(
        "SELECT {TOKEN} FROM locations.ingest_token ORDER BY created_at DESC, id"
    ))
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Ends the token `id` at `at`; `None` when there is none, and a token that was already
/// revoked keeps its first time.
pub(super) async fn revoke_token(
    conn: &mut PgConnection,
    id: IngestTokenId,
    at: DateTime<Utc>,
) -> Result<Option<IngestToken>, sqlx::Error> {
    let row: Option<TokenRow> = sqlx::query_as(&format!(
        "UPDATE locations.ingest_token SET revoked_at = coalesce(revoked_at, $2) \
         WHERE id = $1 RETURNING {TOKEN}"
    ))
    .bind(id)
    .bind(at)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(Into::into))
}

/// The token with this hash that is not revoked, marked as used at `at`.
pub(super) async fn use_token(
    conn: &mut PgConnection,
    hash: &[u8],
    at: DateTime<Utc>,
) -> Result<Option<IngestToken>, sqlx::Error> {
    let row: Option<TokenRow> = sqlx::query_as(&format!(
        "UPDATE locations.ingest_token SET last_used_at = $2 \
         WHERE token_hash = $1 AND revoked_at IS NULL RETURNING {TOKEN}"
    ))
    .bind(hash)
    .bind(at)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(Into::into))
}

/// Adds `fixes` to the count of `token` for `hour`, and keeps `client` as the address of the
/// last post of the hour.
pub(super) async fn count_fixes(
    conn: &mut PgConnection,
    (token, hour): (IngestTokenId, DateTime<Utc>),
    (fixes, client): (i32, std::net::IpAddr),
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO locations.ingest_hour (token_id, hour, fixes, client) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (token_id, hour) DO UPDATE \
         SET fixes = locations.ingest_hour.fixes + excluded.fixes, client = excluded.client",
    )
    .bind(token)
    .bind(hour)
    .bind(fixes)
    .bind(client)
    .execute(conn)
    .await
    .map(drop)
}

/// One hour of one ingest token that has ended.
#[derive(sqlx::FromRow)]
pub(super) struct ClosedHour {
    pub token_id: IngestTokenId,
    pub name: String,
    pub hour: DateTime<Utc>,
    pub fixes: i32,
    pub client: std::net::IpAddr,
}

/// Takes each count of an hour before `open_hour` out of the table, oldest first. A second
/// caller at the same time skips the rows that this one holds.
pub(super) async fn take_closed_hours(
    conn: &mut PgConnection,
    open_hour: DateTime<Utc>,
) -> Result<Vec<ClosedHour>, sqlx::Error> {
    sqlx::query_as(
        "WITH closed AS (SELECT token_id, hour FROM locations.ingest_hour WHERE hour < $1 \
         ORDER BY hour, token_id FOR UPDATE SKIP LOCKED), \
         taken AS (DELETE FROM locations.ingest_hour h USING closed c \
         WHERE h.token_id = c.token_id AND h.hour = c.hour \
         RETURNING h.token_id, h.hour, h.fixes, h.client) \
         SELECT t.token_id, k.name, t.hour, t.fixes, t.client FROM taken t \
         JOIN locations.ingest_token k ON k.id = t.token_id ORDER BY t.hour, t.token_id",
    )
    .bind(open_hour)
    .fetch_all(conn)
    .await
}

fn ids(end: &End) -> (Option<Uuid>, Option<Uuid>, Option<Uuid>) {
    (
        end.device.map(Uuid::from),
        end.site.map(Uuid::from),
        end.place.map(Uuid::from),
    )
}

pub(super) async fn insert_context(
    conn: &mut PgConnection,
    context: &StoredContext,
) -> Result<(), sqlx::Error> {
    let (source, destination) = (ids(&context.source), ids(&context.destination));
    sqlx::query(
        "INSERT INTO locations.transfer_context \
         (event_id, occurred_at, direction, source_device, source_site, source_place, \
          destination_device, destination_site, destination_place, actor_kind, actor, \
          client_device) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(context.event)
    .bind(context.occurred_at)
    .bind(context.direction.as_str())
    .bind(source.0)
    .bind(source.1)
    .bind(source.2)
    .bind(destination.0)
    .bind(destination.1)
    .bind(destination.2)
    .bind(context.actor.kind.as_str())
    .bind(context.actor.name.as_deref())
    .bind(context.actor.client_device.map(Uuid::from))
    .execute(conn)
    .await
    .map(drop)
}

#[derive(sqlx::FromRow)]
struct ContextRow {
    event_id: EventId,
    occurred_at: DateTime<Utc>,
    direction: String,
    source_device: Option<Uuid>,
    source_site: Option<Uuid>,
    source_place: Option<Uuid>,
    destination_device: Option<Uuid>,
    destination_site: Option<Uuid>,
    destination_place: Option<Uuid>,
    actor_kind: String,
    actor: Option<String>,
    client_device: Option<Uuid>,
}

fn end(device: Option<Uuid>, site: Option<Uuid>, place: Option<Uuid>) -> End {
    End {
        device: device.map(Into::into),
        site: site.map(Into::into),
        place: place.map(Into::into),
    }
}

/// The stored context of the transfer event `event`.
pub(super) async fn context(
    db: impl PgExecutor<'_>,
    event: EventId,
) -> Result<Option<StoredContext>, sqlx::Error> {
    let row: Option<ContextRow> = sqlx::query_as(
        "SELECT event_id, occurred_at, direction, source_device, source_site, source_place, \
         destination_device, destination_site, destination_place, actor_kind, actor, \
         client_device FROM locations.transfer_context WHERE event_id = $1",
    )
    .bind(event)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let unreadable = |column: &str| sqlx::Error::Decode(format!("bad context {column}").into());
    Ok(Some(StoredContext {
        event: row.event_id,
        occurred_at: row.occurred_at,
        direction: super::context::Direction::parse(&row.direction)
            .ok_or_else(|| unreadable("direction"))?,
        source: end(row.source_device, row.source_site, row.source_place),
        destination: end(
            row.destination_device,
            row.destination_site,
            row.destination_place,
        ),
        actor: Actor {
            kind: super::context::ActorKind::parse(&row.actor_kind)
                .ok_or_else(|| unreadable("actor"))?,
            name: row.actor,
            client_device: row.client_device.map(Into::into),
        },
    }))
}
