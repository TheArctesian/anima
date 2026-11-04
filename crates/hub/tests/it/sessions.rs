//! The sessions domain: its tables, its face and its GraphQL part (WP-10 T1), and starting
//! and ending sessions with tunnel tickets (WP-10 T2).

mod end;
mod face;
mod graphql_read;
mod guard;
mod harness;
mod host_key_rows;
mod k1;
mod leak;
mod privileges;
mod start;
mod starting;
mod tables;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

const OPERATOR_ID: &str = "0190a000-0000-7000-8000-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OLLAMA: &str = "0190a000-0000-7000-8002-000000000003";

/// Two real OpenSSH public keys (throwaway, made with `ssh-keygen`; the private halves were
/// deleted) and the fingerprints `ssh-keygen -l` printed for them.
const ED25519: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIPTJnBUBJyxtaF5R3VPI+WZbUsZP0DeocOVAgFsBtx/m";
const ED25519_FINGERPRINT: &str = "SHA256:4eECQSeWmB0ww/ND7ZkydOxBUAgciI4AwfhKt8R8Nbk";
const ECDSA: &str = "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBN6Al4EgiR+faDnuIVjWmW1yCwStnaO+iRCpsRc7Py09CTCdHczEiAwuagxLahrlBz/ZDfvgKxIt9rS8Rv3xTDM=";
const ECDSA_FINGERPRINT: &str = "SHA256:5B0BaMRWDnvpdw0F6yEj4v4XY3LfY9f26fSNr+SrSgQ";

/// The instant the tests start sessions at.
fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

/// The session id for a small number, so tests can name rows.
fn session_id(n: u128) -> Uuid {
    Uuid::from_u128(0x0190_a000_0000_7000_8010_0000_0000_0000 + n)
}

/// A session row as the hub would write it.
struct Row {
    id: u128,
    device: &'static str,
    started_minutes: i64,
    /// Minutes after `now()` and the stored reason, or `None` for a live session.
    end: Option<(i64, &'static str)>,
}

impl Row {
    const fn live(id: u128, device: &'static str, started_minutes: i64) -> Self {
        Self {
            id,
            device,
            started_minutes,
            end: None,
        }
    }

    const fn ended(id: u128, device: &'static str, started: i64, reason: &'static str) -> Self {
        Self {
            id,
            device,
            started_minutes: started,
            end: Some((started + 5, reason)),
        }
    }
}

async fn insert(pool: &PgPool, row: &Row) -> Result<(), sqlx::Error> {
    let at = |minutes| now() + TimeDelta::minutes(minutes);
    sqlx::query(
        "insert into sessions.sessions (id, device_id, device_name, protocol, operator_id, \
                operator_name, client_addr, started_at, ended_at, end_reason) \
         select $1, d.id, d.name, 'ssh', $3::text::uuid, 'operator', '10.10.0.77', $4, $5, $6 \
         from devices.devices d where d.id = $2::text::uuid",
    )
    .bind(session_id(row.id))
    .bind(row.device)
    .bind(OPERATOR_ID)
    .bind(at(row.started_minutes))
    .bind(row.end.map(|(minutes, _)| at(minutes)))
    .bind(row.end.map(|(_, reason)| reason))
    .execute(pool)
    .await
    .map(|_| ())
}

async fn insert_all(pool: &PgPool, rows: &[Row]) {
    for row in rows {
        insert(pool, row).await.unwrap();
    }
}

/// The K6 operator, so a host key can name who accepted it.
async fn add_operator(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR_ID)
    .execute(pool)
    .await
    .unwrap();
}

/// Stores an accepted host key: `(algorithm, base64 key, fingerprint)`.
async fn put_host_key(
    pool: &PgPool,
    device: &str,
    (algorithm, key, fingerprint): (&str, &str, &str),
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, $2, $3, $4, $5, $5 + interval '1 minute', $6::text::uuid)",
    )
    .bind(device)
    .bind(algorithm)
    .bind(fingerprint)
    .bind(key)
    .bind(now())
    .bind(OPERATOR_ID)
    .execute(pool)
    .await
    .map(|_| ())
}

const ED25519_KEY: (&str, &str, &str) = ("ssh-ed25519", ED25519, ED25519_FINGERPRINT);
const ECDSA_KEY: (&str, &str, &str) = ("ecdsa-sha2-nistp256", ECDSA, ECDSA_FINGERPRINT);

fn constraint_of(error: &sqlx::Error) -> Option<&str> {
    error.as_database_error().and_then(|e| e.constraint())
}

async fn delete_device(pool: &PgPool, device: &str) {
    sqlx::query("delete from devices.devices where id = $1::text::uuid")
        .bind(device)
        .execute(pool)
        .await
        .unwrap();
}
