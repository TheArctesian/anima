//! When a write fails at commit, no ticket is lost or left behind (D-2, D-10): `start`
//! withdraws the ticket it issued, and `endSession` and the sweep put back what they took, so
//! a later sweep still ends the row `TIMEOUT`.

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use super::{ED25519, ED25519_FINGERPRINT};
use crate::{
    clock::{Clock, FixedClock},
    events::{ClientAddr, Operator},
    ids::{DeviceId, OperatorId},
    sessions::{
        Actor, DisplayRequest, Error, Live, Protocol, StartRequest, Ticket,
        end::{end_session, sweep},
        start,
        tests::fixture::no_recording,
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
    }
}

fn client() -> ClientAddr {
    ClientAddr("10.10.0.77".parse().unwrap())
}

/// The K6 operator and an accepted host key for PVE-01.
async fn ready(pool: &PgPool) {
    sqlx::query(
        "insert into access.operators (id, username, password_hash, created_at) \
         values ($1::text::uuid, 'operator', '$argon2id$x', now())",
    )
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into trust.host_keys (device_id, algorithm, fingerprint_sha256, public_key, \
                first_seen_at, accepted_at, accepted_by) \
         values ($1::text::uuid, 'ssh-ed25519', $2, $3, $4, $4, $5::text::uuid)",
    )
    .bind(PVE_01)
    .bind(ED25519_FINGERPRINT)
    .bind(ED25519)
    .bind(now())
    .bind(OPERATOR)
    .execute(pool)
    .await
    .unwrap();
}

/// Makes every later transaction that `event`s a session row fail at commit.
async fn fail_commits_on(pool: &PgPool, event: &str) {
    sqlx::query(
        "create function public.refuse_commit() returns trigger language plpgsql \
         as $$ begin raise exception 'refused at commit'; end $$",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "create constraint trigger refuse_commit after {event} on sessions.sessions \
         deferrable initially deferred for each row execute function public.refuse_commit()"
    ))
    .execute(pool)
    .await
    .unwrap();
}

async fn start_on_pve_01(pool: &PgPool, live: &Live) -> Result<Ticket, Error> {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let request = StartRequest {
        device: PVE_01.parse::<DeviceId>().unwrap(),
        protocol: Protocol::Ssh,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: None,
        },
        wake: None,
        recording: no_recording(),
    };
    start(pool, &clock, live, &operator(), client(), request).await
}

async fn end_reason(pool: &PgPool, ticket: &Ticket) -> Option<String> {
    sqlx::query_scalar("select end_reason from sessions.sessions where id = $1")
        .bind(ticket.session)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Long after every ticket here expired.
fn later() -> DateTime<Utc> {
    now() + TimeDelta::hours(1)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_a_start_whose_commit_fails_keeps_a_spent_ticket_for_the_sweep(pool: PgPool) {
    ready(&pool).await;
    fail_commits_on(&pool, "insert").await;
    let live = Live::default();
    assert!(matches!(
        start_on_pve_01(&pool, &live).await,
        Err(Error::Database(_))
    ));
    // The commit may have landed before it failed, so the ticket stays, without its token,
    // until it expires: the sweep then ends any row it left (no row: nothing to end).
    assert_eq!(live.take_expired(now()).len(), 0, "kept until it expires");
    assert_eq!(live.take_expired(later()).len(), 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn ra_008_an_end_session_whose_write_fails_puts_the_ticket_back(pool: PgPool) {
    ready(&pool).await;
    let live = Live::default();
    let ticket = start_on_pve_01(&pool, &live).await.unwrap();
    fail_commits_on(&pool, "update").await;
    let by = Actor {
        operator: operator(),
        client: client(),
        at: now(),
    };
    assert!(
        end_session(&pool, &live, ticket.session, &by)
            .await
            .is_err()
    );
    assert_eq!(end_reason(&pool, &ticket).await, None);
    let back = live.take_expired(later());
    assert_eq!(back.len(), 1, "the sweep still finds it");
    assert_eq!(back[0].session, ticket.session);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn api_009_a_sweep_whose_write_fails_puts_the_tickets_back(pool: PgPool) {
    ready(&pool).await;
    let live = Live::default();
    let ticket = start_on_pve_01(&pool, &live).await.unwrap();
    fail_commits_on(&pool, "update").await;
    assert!(sweep(&pool, &live, later()).await.is_err());
    assert_eq!(end_reason(&pool, &ticket).await, None);

    sqlx::query("drop trigger refuse_commit on sessions.sessions")
        .execute(&pool)
        .await
        .unwrap();
    sweep(&pool, &live, later()).await.unwrap();
    assert_eq!(end_reason(&pool, &ticket).await.as_deref(), Some("timeout"));
}
