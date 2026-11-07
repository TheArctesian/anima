//! The opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1, RA-031): it
//! waits past the usual limit, trying the device again until it answers, and ends at the
//! woken limit when it never does; without the packet the opening does not wait for a device
//! that does not answer. The deny list holds on every try (R-030).

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::super::{
    super::super::opening::{self, NotOpened, Opening},
    bench::{Bench, END_WAIT, Setup, bench},
    socket::open_with,
};
use crate::{
    sessions::{
        Live, Presented, Redeemed,
        tests::fixture::{
            LOOPBACK, address, ended, ended_once, move_device, now, reissued_woken, waking_port,
        },
    },
    targets::AddressPolicy,
};

/// The woken limit of these tests: far longer than any refusal takes.
const WOKEN: Duration = Duration::from_secs(10);

/// The usual opening limit, shortened; the waking device answers only well after it.
const USUAL: Duration = Duration::from_millis(200);
/// When the waking device starts to answer.
const ANSWERS_AFTER: Duration = Duration::from_millis(800);

/// A bench whose opening limit is [`USUAL`], or 10 s after a wake, and whose device answers
/// only after [`ANSWERS_AFTER`], forwarding to the fake SSH server.
async fn waking(pool: &PgPool) -> Bench {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.opening = USUAL;
        terminal.timing.woken_opening = WOKEN;
        terminal.timing.reconnect = Duration::from_millis(50);
    });
    let bench = bench(pool, setup).await;
    let port = waking_port(ANSWERS_AFTER, Some(bench.ssh.port));
    move_device(pool, LOOPBACK, port).await;
    bench
}

/// The bench's ticket, redeemed, marked as woken or not.
fn redeemed(bench: &Bench, wake_sent: bool) -> Redeemed {
    let mut redeemed = bench
        .live
        .redeem(Presented::terminal(&bench.token, address(LOOPBACK)), now())
        .unwrap();
    redeemed.ticket.wake_sent = wake_sent;
    redeemed
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_woken_opening_waits_past_the_usual_limit_until_the_device_answers(pool: PgPool) {
    let bench = waking(&pool).await;
    let redeemed = redeemed(&bench, true);
    let begun = Instant::now();
    let opened = opening::open(&bench.terminal, &redeemed.ticket).await;
    assert!(
        matches!(opened, Ok(Opening::SignedIn(_))),
        "the device opens"
    );
    assert!(begun.elapsed() >= ANSWERS_AFTER, "it waited for the device");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_without_a_wake_the_opening_does_not_wait_for_the_device(pool: PgPool) {
    let bench = waking(&pool).await;
    let redeemed = redeemed(&bench, false);
    let begun = Instant::now();
    let opened = opening::open(&bench.terminal, &redeemed.ticket).await;
    assert!(matches!(opened, Err(NotOpened::Unreachable)));
    assert!(begun.elapsed() < ANSWERS_AFTER);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_woken_device_that_never_answers_ends_the_opening_at_the_woken_limit(
    pool: PgPool,
) {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.opening = USUAL;
        terminal.timing.woken_opening = Duration::from_millis(400);
        terminal.timing.reconnect = Duration::from_millis(50);
    });
    let bench = bench(&pool, setup).await;
    let port = waking_port(Duration::from_secs(3600), None);
    move_device(&pool, LOOPBACK, port).await;
    let redeemed = redeemed(&bench, true);
    let opened = opening::open(&bench.terminal, &redeemed.ticket).await;
    assert!(matches!(opened, Err(NotOpened::Timeout)));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_a_woken_ticket_to_a_denied_address_is_503_at_once_and_never_reached(pool: PgPool) {
    let policy = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]);
    let setup = Setup {
        live: Live::with_tunnel_end_wait(END_WAIT)
            .reaching_loopback_for_tests()
            .denying(policy),
        ..Setup::terminal(|terminal| {
            terminal.timing.woken_opening = WOKEN;
            terminal.timing.reconnect = Duration::from_millis(50);
        })
    };
    let bench = bench(&pool, setup).await;
    let device = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    move_device(&pool, LOOPBACK, device.local_addr().unwrap().port()).await;
    let token = reissued_woken(&bench.live, bench.session);
    let begun = Instant::now();
    let url = format!("ws://{}/terminal?token={token}", bench.served);
    let refused = open_with(&url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    assert!(
        begun.elapsed() < WOKEN / 2,
        "refused before the woken limit"
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert!(ended.events[0].3.contains("ANIMA_PROBE_DENY"), "{ended:?}");
    let reached = timeout(Duration::from_millis(50), device.accept()).await;
    assert!(reached.is_err(), "no connection reached the device");
}
