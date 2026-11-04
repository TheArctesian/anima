//! The store's writes: a session row starts live and ends once; the runtime role can do all
//! of it through migration 0001's default privileges alone (RA-008, R-016, K2 "Roles").

use std::net::IpAddr;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

use super::ED25519;
use crate::{
    events::Operator,
    ids::{DeviceId, OperatorId, SessionId},
    sessions::{
        EndReason, Protocol, SessionEnd,
        model::SessionFilter,
        store::{self, NewSession},
    },
};

const OPERATOR: &str = "0190a000-0000-7000-8000-000000000001";
const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const DOCKER_HOST: &str = "0190a000-0000-7000-8002-000000000002";
const OLLAMA: &str = "0190a000-0000-7000-8002-000000000003";

fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn operator() -> Operator {
    Operator {
        id: OPERATOR.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
    }
}

fn device(id: &str) -> DeviceId {
    id.parse().unwrap()
}

async fn start(conn: &mut PgConnection, id: u128, minutes: i64) -> SessionId {
    let operator = operator();
    let new = NewSession {
        id: SessionId::from(uuid::Uuid::from_u128(id)),
        device: device(PVE_01),
        device_name: "pve-01",
        protocol: Protocol::Ssh,
        operator: &operator,
        client: "10.10.0.77".parse::<IpAddr>().unwrap(),
        started_at: now() + TimeDelta::minutes(minutes),
    };
    let session = store::insert(&mut *conn, &new).await.unwrap();
    assert!(session.is_live());
    assert_eq!(session.operator_name, "operator");
    session.id
}

async fn add_operator(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
}

async fn put_host_key(conn: &mut PgConnection, device: &str) {
    sqlx::query(
        "insert into sessions.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, 'ssh-ed25519', $2, $3, $4, $4, $5::text::uuid) \
         on conflict (device_id) do update set accepted_at = excluded.accepted_at",
    )
    .bind(device)
    .bind(super::ED25519_FINGERPRINT)
    .bind(ED25519)
    .bind(now())
    .bind(OPERATOR)
    .execute(conn)
    .await
    .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn ra_008_a_session_ends_once_and_the_first_end_wins(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    let id = start(&mut conn, 1, 0).await;
    let first = SessionEnd {
        at: now() + TimeDelta::minutes(5),
        reason: EndReason::Operator,
    };
    let later = SessionEnd {
        at: now() + TimeDelta::minutes(6),
        reason: EndReason::Timeout,
    };
    assert!(store::end(&mut *conn, id, first).await.unwrap());
    assert!(!store::end(&mut *conn, id, later).await.unwrap());

    let page = store::page(&mut *conn, SessionFilter::default(), 10, None)
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].end, Some(first));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn log_006_r_016_runtime_role_reads_and_writes_session_tables(pool: PgPool) {
    add_operator(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("set role anima_runtime")
        .execute(&mut *conn)
        .await
        .unwrap();

    let id = start(&mut conn, 2, 0).await;
    start(&mut conn, 3, 1).await;
    let end = SessionEnd {
        at: now() + TimeDelta::minutes(2),
        reason: EndReason::RemoteClosed,
    };
    assert!(store::end(&mut *conn, id, end).await.unwrap());
    assert_eq!(store::close_stale(&mut *conn, now()).await.unwrap(), 1);
    let page = store::page(&mut *conn, SessionFilter::default(), 10, None)
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 2);

    put_host_key(&mut conn, PVE_01).await;
    put_host_key(&mut conn, PVE_01).await;
    let keys = store::host_keys(&mut *conn, &[device(PVE_01)])
        .await
        .unwrap();
    assert_eq!(keys.len(), 1);
}

/// Index and sequential scans of `sessions.host_keys` so far in this transaction.
async fn host_key_scans(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar(
        "select coalesce(seq_scan, 0) + coalesce(idx_scan, 0) from pg_stat_xact_user_tables \
         where schemaname = 'sessions' and relname = 'host_keys'",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn api_005_host_keys_reads_many_devices_in_one_query(pool: PgPool) {
    add_operator(&pool).await;
    let mut conn = pool.acquire().await.unwrap();
    put_host_key(&mut conn, PVE_01).await;
    put_host_key(&mut conn, DOCKER_HOST).await;
    let devices = [device(PVE_01), device(DOCKER_HOST), device(OLLAMA)];

    let mut tx = pool.begin().await.unwrap();
    let keys = store::host_keys(&mut *tx, &devices).await.unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(host_key_scans(&mut tx).await, 1);
    // The measure can tell: one lookup per device scans once per device.
    for one in devices {
        store::host_keys(&mut *tx, &[one]).await.unwrap();
    }
    assert_eq!(host_key_scans(&mut tx).await, 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_005_host_keys_of_no_devices_runs_no_query(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    // After an error PostgreSQL refuses every statement until the transaction ends, so any
    // query on this connection would fail.
    sqlx::query("select 1 / 0")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    let keys = store::host_keys(&mut *tx, &[]).await.unwrap();
    assert!(keys.is_empty());
    assert!(store::host_keys(&mut *tx, &[device(PVE_01)]).await.is_err());
}
