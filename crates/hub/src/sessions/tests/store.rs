//! The store's writes: a session row starts live and ends once; the runtime role can do all
//! of it through migration 0001's default privileges alone (RA-008, R-016, K2 "Roles").

use std::net::IpAddr;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{PgConnection, PgPool};

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

fn now() -> DateTime<Utc> {
    "2026-11-04T09:00:00Z".parse().unwrap()
}

fn operator() -> Operator {
    Operator {
        id: OPERATOR.parse::<OperatorId>().unwrap(),
        name: "operator".to_owned(),
        actor: crate::events::Actor::for_tests(),
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
    assert!(store::end(&mut *conn, id, first).await.unwrap().is_some());
    assert!(store::end(&mut *conn, id, later).await.unwrap().is_none());

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
async fn mv_003_an_end_before_the_start_is_written_at_the_start(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    // Started 5 minutes ahead of a clock that then stepped back.
    let id = start(&mut conn, 1, 5).await;
    let end = SessionEnd {
        at: now(),
        reason: EndReason::RemoteClosed,
    };
    assert!(store::end(&mut *conn, id, end).await.unwrap().is_some());
    let session = store::get(&mut *conn, id).await.unwrap().unwrap();
    let at_start = SessionEnd {
        at: now() + TimeDelta::minutes(5),
        ..end
    };
    assert_eq!(session.end, Some(at_start));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn log_006_r_016_runtime_role_reads_and_writes_session_tables(pool: PgPool) {
    add_operator(&pool).await;
    // Detached: a connection whose role changed must not return to the pool (R-019).
    let mut conn = pool.acquire().await.unwrap().detach();
    sqlx::query("set role anima_runtime")
        .execute(&mut conn)
        .await
        .unwrap();

    let id = start(&mut conn, 2, 0).await;
    start(&mut conn, 3, 1).await;
    let end = SessionEnd {
        at: now() + TimeDelta::minutes(2),
        reason: EndReason::RemoteClosed,
    };
    assert!(store::end(&mut conn, id, end).await.unwrap().is_some());
    let stale = store::close_stale(&mut conn, now() + TimeDelta::minutes(3))
        .await
        .unwrap();
    assert_eq!(stale.len(), 1);
    let page = store::page(&mut conn, SessionFilter::default(), 10, None)
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 2);
}
