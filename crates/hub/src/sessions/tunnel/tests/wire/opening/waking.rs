//! The opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1, RA-031): it
//! waits, past the usual limit, until the device's port answers before it asks guacd to
//! connect, and gives up at the woken limit. The deny list holds on every try (R-030).

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};
use tokio_tungstenite::tungstenite::http::StatusCode;

use super::super::{
    super::super::opening::{self, NotOpened},
    bench::{Bench, LOOPBACK, Setup, bench, now, on_tunnel},
    ended_once,
    fake_guacd::{FakeGuacd, RDP_ARGS, Script, fake_guacd},
    records::ended,
    socket::open_with,
};
use crate::{
    sessions::{
        Live, Protocol, Redeemed,
        tests::fixture::{DOCKER_HOST, reissued_woken, waking_port},
        tickets::WOKEN_OPENING,
    },
    targets::AddressPolicy,
};

/// The usual limit of each opening step, shortened; the waking device answers well after it.
const USUAL: Duration = Duration::from_millis(200);
/// When the waking device starts to answer.
const ANSWERS_AFTER: Duration = Duration::from_millis(800);

/// A guacd that serves RDP, and a bench in front of it whose woken opening may wait `woken`
/// for a device that answers only after [`ANSWERS_AFTER`].
async fn waking(pool: &PgPool, woken: Duration) -> (FakeGuacd, Bench) {
    let guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let setup = Setup::route(move |route| {
        route.timing.connect = USUAL;
        route.timing.woken_opening = woken;
        route.timing.reconnect = Duration::from_millis(50);
    });
    let bench = bench(pool, &guacd.address, setup).await;
    move_rdp_port(pool, waking_port(ANSWERS_AFTER, None)).await;
    (guacd, bench)
}

/// Points `docker-host`'s RDP port at `port`.
async fn move_rdp_port(pool: &PgPool, port: u16) {
    sqlx::query("update devices.rdp_settings set port = $2 where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
}

/// The bench's ticket, redeemed and marked as woken.
fn woken(bench: &Bench) -> Redeemed {
    let mut redeemed = bench.live.redeem(on_tunnel(&bench.token), now()).unwrap();
    redeemed.ticket.wake_sent = true;
    redeemed
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_woken_opening_reaches_guacd_once_the_device_answers(pool: PgPool) {
    let (_guacd, bench) = waking(&pool, Duration::from_secs(10)).await;
    let redeemed = woken(&bench);
    let begun = Instant::now();
    let opened = opening::open(&bench.route, &redeemed.ticket).await;
    assert!(opened.is_ok(), "the gateway side opens");
    assert!(begun.elapsed() >= ANSWERS_AFTER, "it waited for the device");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_woken_device_that_never_answers_ends_the_opening_at_the_woken_limit(
    pool: PgPool,
) {
    let (_guacd, bench) = waking(&pool, Duration::from_millis(400)).await;
    let redeemed = woken(&bench);
    let opened = opening::open(&bench.route, &redeemed.ticket).await;
    assert!(matches!(opened, Err(NotOpened::NotAwake)));
}

#[test]
fn ra_031_the_not_awake_message_names_the_woken_limit() {
    let seconds = format!("within {} seconds", WOKEN_OPENING.as_secs());
    for protocol in [Protocol::Rdp, Protocol::Vnc] {
        let message = NotOpened::NotAwake.message(protocol);
        assert!(message.contains(&seconds), "{message}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_030_a_woken_ticket_to_a_denied_address_is_503_at_once_and_never_reached(pool: PgPool) {
    let mut guacd = fake_guacd(Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    })
    .await;
    let woken = Duration::from_secs(10);
    let policy = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]);
    let setup = Setup {
        live: Live::default()
            .reaching_loopback_for_tests()
            .denying(policy),
        ..Setup::route(move |route| {
            route.timing.woken_opening = woken;
            route.timing.reconnect = Duration::from_millis(50);
        })
    };
    let bench = bench(&pool, &guacd.address, setup).await;
    let device = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    move_rdp_port(&pool, device.local_addr().unwrap().port()).await;
    let token = reissued_woken(&bench.live, bench.session);
    let begun = Instant::now();
    let url = format!("ws://{}/tunnel?token={token}", bench.served);
    let refused = open_with(&url, true).await.err();
    assert_eq!(refused, Some(StatusCode::SERVICE_UNAVAILABLE));
    assert!(
        begun.elapsed() < woken / 2,
        "refused before the woken limit"
    );
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "gateway_error", "warning", LOOPBACK);
    assert!(ended.events[0].3.contains("ANIMA_PROBE_DENY"), "{ended:?}");
    let reached = timeout(Duration::from_millis(50), device.accept()).await;
    assert!(reached.is_err(), "no connection reached the device");
    assert!(guacd.selected.try_recv().is_err(), "guacd is never reached");
}
