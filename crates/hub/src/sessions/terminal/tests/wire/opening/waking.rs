//! The opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1, RA-031): it
//! waits past the usual limit, trying the device again until it answers, and ends at the
//! woken limit when it never does; without the packet the opening does not wait for a device
//! that does not answer. The deny list holds on every try (R-030).

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::{net::TcpListener, sync::oneshot, time::timeout};
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
            LOOPBACK, address, answer_after, ended, ended_once, move_device, now, reissued_woken,
            waking_port,
        },
    },
    targets::AddressPolicy,
};

/// The woken limit of these tests: far longer than any refusal takes, so only a bug can end
/// a woken opening of a device that answers.
const WOKEN: Duration = Duration::from_secs(10);

/// The usual opening limit, shortened.
const USUAL: Duration = Duration::from_millis(200);
/// How long after the opening began the waking device answers: three usual limits, and far
/// inside [`WOKEN`].
const ANSWERS_AFTER: Duration = Duration::from_millis(600);

/// A bench whose opening limit is [`USUAL`], or [`WOKEN`] after a wake, and whose device is
/// asleep until the returned sender wakes it, forwarding to the fake SSH server.
async fn waking(pool: &PgPool) -> (Bench, oneshot::Sender<()>) {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.opening = USUAL;
        terminal.timing.woken_opening = WOKEN;
        terminal.timing.reconnect = Duration::from_millis(50);
    });
    let bench = bench(pool, setup).await;
    let (port, answer) = waking_port(Some(bench.ssh.port));
    move_device(pool, LOOPBACK, port).await;
    (bench, answer)
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
    let (bench, answer) = waking(&pool).await;
    let redeemed = redeemed(&bench, true);
    let (began, was_begun) = oneshot::channel();
    let opening = async {
        began.send(()).unwrap();
        opening::open(&bench.terminal, &redeemed.ticket).await
    };
    let (opened, ()) = tokio::join!(opening, answer_after(was_begun, ANSWERS_AFTER, answer));
    assert!(
        matches!(opened, Ok(Opening::SignedIn(_))),
        "the device opens although it answered after the usual limit"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_without_a_wake_the_opening_does_not_wait_for_the_device(pool: PgPool) {
    let (bench, _asleep) = waking(&pool).await;
    let redeemed = redeemed(&bench, false);
    let opened = opening::open(&bench.terminal, &redeemed.ticket).await;
    assert!(
        matches!(opened, Err(NotOpened::Unreachable)),
        "refused at once, not waiting for a device that is asleep"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_woken_device_that_never_answers_ends_the_opening_at_the_woken_limit(
    pool: PgPool,
) {
    const LIMIT: Duration = Duration::from_millis(600);
    const NEVER: Duration = Duration::from_secs(30);
    let setup = Setup::terminal(|terminal| {
        terminal.timing.opening = USUAL;
        terminal.timing.woken_opening = LIMIT;
        terminal.timing.reconnect = Duration::from_millis(50);
    });
    let bench = bench(&pool, setup).await;
    let (port, _asleep) = waking_port(None);
    move_device(&pool, LOOPBACK, port).await;
    let redeemed = redeemed(&bench, true);
    let begun = Instant::now();
    let opened = timeout(NEVER, opening::open(&bench.terminal, &redeemed.ticket))
        .await
        .expect("the opening ends");
    assert!(matches!(opened, Err(NotOpened::Timeout)));
    assert!(
        begun.elapsed() >= LIMIT,
        "it waited past the usual limit, to the woken one"
    );
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
