//! The session task owns the ticket from redeem to end (MV-003, SEC-002): a client that
//! leaves during the opening, an `endSession` during the opening, and a task that panics
//! each end the session once; the known gap of a client that leaves after a stray byte; and
//! the end's write is retried once (LOG-005).

mod waking;

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    time::timeout,
};

use super::{
    super::super::ending::{Cause, End},
    Bench,
    bench::{
        END_WAIT, LOOPBACK, PASSWORD, Setup, address, bench, ending_within_end_wait, now,
        on_tunnel, operator,
    },
    ended_once,
    fake_guacd::{Act, FakeGuacd, RDP_ARGS, Script, fake_guacd},
    records::ended,
};
use crate::sessions::{
    Actor,
    end::{Unfinished, end_session, finish},
    upgrade::FarSide as _,
};

/// A guacd that takes `select` and then says nothing until told, and a bench in front of it.
async fn stalled(pool: &PgPool, setup: Setup) -> (FakeGuacd, Bench) {
    let guacd = fake_guacd(Script::StallBeforeArgs { args: RDP_ARGS }).await;
    let bench = bench(pool, &guacd.address, setup).await;
    (guacd, bench)
}

/// Sends the upgrade request as guacamole-common-js would and keeps the connection, then
/// waits until the hub has sent guacd `select`: the opening is under way.
async fn request_reaching_guacd(bench: &Bench, guacd: &mut FakeGuacd) -> TcpStream {
    let mut raw = TcpStream::connect(bench.served).await.unwrap();
    let request = format!(
        "GET /tunnel?token={} HTTP/1.1\r\nHost: {}\r\nConnection: Upgrade\r\n\
         Upgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Protocol: guacamole\r\n\r\n",
        bench.token, bench.served
    );
    raw.write_all(request.as_bytes()).await.unwrap();
    timeout(Duration::from_secs(5), &mut guacd.selected)
        .await
        .expect("the opening reaches guacd")
        .unwrap();
    raw
}

/// The status line of the response on `raw`.
async fn status_line(raw: &mut TcpStream) -> String {
    let mut response = [0; 64];
    let read = timeout(Duration::from_secs(5), raw.read(&mut response))
        .await
        .unwrap()
        .unwrap();
    let text = String::from_utf8_lossy(&response[..read]).into_owned();
    text.lines().next().unwrap_or_default().to_owned()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_client_that_leaves_during_the_opening_ends_the_session_operator(pool: PgPool) {
    // The opening would wait out the whole handshake if nothing noticed the client leave.
    let setup = Setup::route(|route| route.timing.handshake = Duration::from_secs(60));
    let (mut guacd, bench) = stalled(&pool, setup).await;
    let raw = request_reaching_guacd(&bench, &mut guacd).await;
    drop(raw);
    // The client left first, so the session ends `OPERATOR` as for any client that leaves;
    // the opening stops and guacd gets no credential.
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client left before the RDP session opened."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_d_10_end_session_during_the_opening_sends_guacd_no_credential(pool: PgPool) {
    // `endSession` waits for the tunnel to end its row (D-10), up to `END_WAIT`; the handshake
    // wait is far longer, so a hub that ignored the signal would still be in its handshake
    // when guacd answers below. A fixed short wait here raced the tunnel's end write on a
    // loaded machine.
    let handshake = Duration::from_secs(60);
    assert!(END_WAIT < handshake);
    let setup = Setup {
        route: Box::new(move |route| route.timing.handshake = handshake),
        ..ending_within_end_wait()
    };
    let (mut guacd, bench) = stalled(&pool, setup).await;
    let mut raw = request_reaching_guacd(&bench, &mut guacd).await;
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    let row = end_session(&pool, &bench.live, bench.session, &by)
        .await
        .unwrap();
    // Only now does guacd offer its `args`: a hub still in the handshake would answer with
    // the credential.
    guacd.act.send(Act::Close).ok();
    let after_args = timeout(Duration::from_secs(5), guacd.handshake)
        .await
        .unwrap()
        .unwrap();
    assert!(
        after_args.iter().all(|i| i.opcode != "connect"),
        "{after_args:?}"
    );
    let mut sent = after_args.iter().flat_map(|i| &i.args);
    assert!(sent.all(|arg| !arg.contains(PASSWORD)), "no credential");
    assert!(row.end.is_some(), "the tunnel ended its row in time");
    assert_eq!(
        status_line(&mut raw).await,
        "HTTP/1.1 503 Service Unavailable"
    );
    ended_once(
        &ended(&pool, bench.session).await,
        "operator",
        "info",
        "10.10.0.80",
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn mv_003_a_session_task_that_panics_still_ends_the_session(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    let redeemed = bench.live.redeem(on_tunnel(&bench.token), now()).unwrap();
    let records = bench.route.records();
    // As `upgrade::run_session` holds the session from its first line.
    let task = tokio::spawn(async move {
        let _unfinished = Unfinished::new(records, redeemed);
        panic!("the session task fails");
    });
    assert!(task.await.unwrap_err().is_panic());
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The hub's tunnel stopped unexpectedly and closed the RDP session."
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_9_a_session_task_that_panics_after_end_session_ends_with_the_enders_cause(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    let redeemed = bench.live.redeem(on_tunnel(&bench.token), now()).unwrap();
    let by = Actor {
        operator: operator(),
        client: address("10.10.0.80"),
        at: now() + TimeDelta::seconds(5),
    };
    bench.live.end_by(bench.session, &by);
    let records = bench.route.records();
    let task = tokio::spawn(async move {
        let _unfinished = Unfinished::new(records, redeemed);
        panic!("the session task fails after the end signal");
    });
    assert!(task.await.unwrap_err().is_panic());
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", "10.10.0.80");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_002_known_gap_a_client_that_leaves_after_a_stray_byte_is_seen_only_at_the_wait(
    pool: PgPool,
) {
    // Pins today's behaviour (issue #1): hyper does not notice an EOF while it holds an
    // unread byte, so the opening carries on until its own wait runs out.
    let setup = Setup::route(|route| route.timing.handshake = Duration::from_millis(500));
    let (mut guacd, bench) = stalled(&pool, setup).await;
    let mut raw = request_reaching_guacd(&bench, &mut guacd).await;
    raw.write_all(b"x").await.unwrap();
    drop(raw);
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The gateway did not open the RDP session."
    );
}

/// Makes the first transaction that updates a session row fail at commit, and only that one.
async fn fail_first_commit_on_update(pool: &PgPool) {
    for statement in [
        "create sequence public.refusals",
        "create function public.refuse_first_commit() returns trigger language plpgsql \
         as $$ begin if nextval('public.refusals') = 1 then \
         raise exception 'refused at commit'; end if; return null; end $$",
        "create constraint trigger refuse_first_commit after update on sessions.sessions \
         deferrable initially deferred for each row \
         execute function public.refuse_first_commit()",
    ] {
        sqlx::query(statement).execute(pool).await.unwrap();
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn log_005_an_end_whose_first_commit_fails_is_written_by_the_retry(pool: PgPool) {
    let bench = bench(&pool, "127.0.0.1:9", Setup::default()).await;
    let redeemed = bench.live.redeem(on_tunnel(&bench.token), now()).unwrap();
    fail_first_commit_on_update(&pool).await;
    finish(
        &bench.route.records(),
        redeemed,
        &End::from(Cause::OperatorClosed),
    )
    .await;
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
}
