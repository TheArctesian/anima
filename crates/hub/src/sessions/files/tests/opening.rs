//! `openFiles` (K1, K4 Section 9): a files ticket only for a live SSH session of the same
//! operator and client address whose terminal tunnel is open, and none while shutting down.

use sqlx::PgPool;

use super::bench::{bench, by};
use crate::{
    events::Operator,
    ids::{OperatorId, SessionId},
    sessions::{
        Actor, Error, Missing,
        files::opening::open_files,
        tests::fixture::{LOOPBACK, address, now},
        tickets::LIFETIME,
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_live_ssh_session_gets_a_files_ticket(pool: PgPool) {
    let bench = bench(&pool).await;
    let ticket = open_files(&bench.db, &bench.live, &by(LOOPBACK), bench.session)
        .await
        .unwrap();
    assert_eq!(ticket.session, bench.session);
    assert_eq!(ticket.tunnel_path, "/files");
    assert_eq!(ticket.expires_at, now() + LIFETIME);
    assert_eq!(ticket.token().len(), 43);
    assert!(!format!("{ticket:?}").contains(ticket.token()), "LOG-007");
}

/// `openFiles` refuses with `NotFound` for the session.
fn not_found(result: Result<crate::sessions::Ticket, Error>, session: SessionId) {
    match result {
        Err(Error::NotFound(Missing::Session(missing))) => assert_eq!(missing, session),
        other => panic!("{other:?}"),
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_another_operator_or_address_gets_no_ticket(pool: PgPool) {
    let bench = bench(&pool).await;
    let other = Operator {
        id: "0190a000-0000-7000-8000-0000000000ff"
            .parse::<OperatorId>()
            .unwrap(),
        name: "other".to_owned(),
    };
    let (db, live, session) = (&bench.db, &bench.live, bench.session);
    let other = Actor {
        operator: other,
        client: address(LOOPBACK),
        at: now(),
    };
    not_found(open_files(db, live, &other, session).await, session);
    let elsewhere = open_files(db, live, &by("127.0.0.2"), session).await;
    not_found(elsewhere, session);
    let unknown = SessionId::from(uuid::Uuid::now_v7());
    not_found(open_files(db, live, &by(LOOPBACK), unknown).await, unknown);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_a_session_whose_tunnel_ended_gets_no_ticket(pool: PgPool) {
    let mut bench = bench(&pool).await;
    drop(bench.terminal.take());
    let ended = open_files(&bench.db, &bench.live, &by(LOOPBACK), bench.session).await;
    not_found(ended, bench.session);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_032_no_ticket_while_the_hub_shuts_down(pool: PgPool) {
    let bench = bench(&pool).await;
    let _closing = bench.live.shut_down();
    let refused = open_files(&bench.db, &bench.live, &by(LOOPBACK), bench.session).await;
    assert!(matches!(refused, Err(Error::Unavailable)), "{refused:?}");
}
