//! The reads that build an [`OperationalKey`]: its metadata, its versions and its consumers.
//! No statement here selects a sealed value (LOG-007).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::{
    fleet::credentials::operational::{
        lifecycle,
        model::{Consumer, Metadata, OperationalKey, Version, VersionState},
    },
    ids::{DeviceId, OperationalKeyId},
};

#[derive(sqlx::FromRow)]
struct KeyRow {
    id: OperationalKeyId,
    name: String,
    provider: String,
    purpose: String,
    instructions: String,
    provider_link: Option<String>,
    rotation_days: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    created_by: String,
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    key_id: OperationalKeyId,
    number: i32,
    state: String,
    created_at: DateTime<Utc>,
    created_by: String,
    ended_at: Option<DateTime<Utc>>,
    rotated_at_provider: bool,
}

#[derive(sqlx::FromRow)]
struct ConsumerRow {
    key_id: OperationalKeyId,
    device_id: DeviceId,
    workload: String,
    granted_at: DateTime<Utc>,
    granted_by: String,
    held_version: Option<i32>,
    held_seen_at: Option<DateTime<Utc>>,
    fetched_version: Option<i32>,
    fetched_at: Option<DateTime<Utc>>,
}

const KEY_COLUMNS: &str = "id, name, provider, purpose, instructions, provider_link, \
     rotation_days, expires_at, created_at, created_by";

/// The keys that are not removed, by name (`only` none), or the one named `only`, as they read
/// at `now`.
pub(in crate::fleet::credentials::operational) async fn keys(
    conn: &mut PgConnection,
    only: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<OperationalKey>, sqlx::Error> {
    let rows: Vec<KeyRow> = sqlx::query_as(&format!(
        "SELECT {KEY_COLUMNS} FROM credentials.operational_keys \
         WHERE removed_at IS NULL AND ($1::text IS NULL OR name = $1) ORDER BY name"
    ))
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    let ids: Vec<OperationalKeyId> = rows.iter().map(|row| row.id).collect();
    let versions: Vec<VersionRow> = sqlx::query_as(
        "SELECT key_id, number, state, created_at, created_by, ended_at, rotated_at_provider \
         FROM credentials.operational_key_versions WHERE key_id = ANY($1) \
         ORDER BY key_id, number DESC",
    )
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    let consumers: Vec<ConsumerRow> = sqlx::query_as(
        "SELECT key_id, device_id, workload, granted_at, granted_by, held_version, \
         held_seen_at, fetched_version, fetched_at FROM credentials.operational_key_consumers \
         WHERE key_id = ANY($1) ORDER BY key_id, device_id, workload",
    )
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    Ok(assemble(rows, versions, consumers, now))
}

/// The consumers of the key whose daemon held version `number` at its last report.
pub(in crate::fleet::credentials::operational) async fn holders(
    conn: &mut PgConnection,
    key: OperationalKeyId,
    number: i32,
) -> Result<Vec<Consumer>, sqlx::Error> {
    let rows: Vec<ConsumerRow> = sqlx::query_as(
        "SELECT key_id, device_id, workload, granted_at, granted_by, held_version, \
         held_seen_at, fetched_version, fetched_at FROM credentials.operational_key_consumers \
         WHERE key_id = $1 AND held_version = $2 ORDER BY device_id, workload",
    )
    .bind(key)
    .bind(number)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(consumer).collect())
}

fn assemble(
    rows: Vec<KeyRow>,
    versions: Vec<VersionRow>,
    consumers: Vec<ConsumerRow>,
    now: DateTime<Utc>,
) -> Vec<OperationalKey> {
    let mut versions_of: HashMap<OperationalKeyId, Vec<VersionRow>> = HashMap::new();
    for row in versions {
        versions_of.entry(row.key_id).or_default().push(row);
    }
    let mut consumers_of: HashMap<OperationalKeyId, Vec<Consumer>> = HashMap::new();
    for row in consumers {
        consumers_of
            .entry(row.key_id)
            .or_default()
            .push(consumer(row));
    }
    rows.into_iter()
        .map(|row| {
            let versions = versions_of.remove(&row.id).unwrap_or_default();
            let consumers = consumers_of.remove(&row.id).unwrap_or_default();
            key(row, versions, consumers, now)
        })
        .collect()
}

fn key(
    row: KeyRow,
    versions: Vec<VersionRow>,
    consumers: Vec<Consumer>,
    now: DateTime<Utc>,
) -> OperationalKey {
    let metadata = Metadata {
        provider: row.provider,
        purpose: row.purpose,
        instructions: row.instructions,
        provider_link: row.provider_link,
        rotation_days: row.rotation_days,
        expires_at: row.expires_at,
    };
    let last_rotation = versions
        .first()
        .map_or(row.created_at, |newest| newest.created_at);
    let versions = versions
        .into_iter()
        .map(|version| Version {
            number: version.number,
            // The column's check admits only the four states.
            state: lifecycle::read_state(
                VersionState::from_sql(&version.state).unwrap_or(VersionState::Revoked),
                metadata.expires_at,
                now,
            ),
            created_at: version.created_at,
            created_by: version.created_by,
            ended_at: version.ended_at,
            rotated_at_provider: version.rotated_at_provider,
        })
        .collect();
    OperationalKey {
        id: row.id,
        name: row.name,
        due: lifecycle::due(&metadata, last_rotation, now),
        metadata,
        created_at: row.created_at,
        created_by: row.created_by,
        versions,
        consumers,
    }
}

fn consumer(row: ConsumerRow) -> Consumer {
    Consumer {
        device: row.device_id,
        workload: row.workload,
        granted_at: row.granted_at,
        granted_by: row.granted_by,
        held_version: row.held_version,
        held_seen_at: row.held_seen_at,
        fetched_version: row.fetched_version,
        fetched_at: row.fetched_at,
    }
}
