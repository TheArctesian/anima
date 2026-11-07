//! The opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1, RA-031): it
//! waits, past the usual limit, until the device's port answers before it asks guacd to
//! connect, and gives up at the woken limit.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use super::super::{
    super::super::opening::{self, NotOpened},
    bench::{Bench, Setup, bench, now, on_tunnel},
    fake_guacd::{FakeGuacd, RDP_ARGS, Script, fake_guacd},
};
use crate::sessions::{
    Redeemed,
    tests::fixture::{DOCKER_HOST, waking_port},
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
    let port = waking_port(ANSWERS_AFTER, None);
    sqlx::query("update devices.rdp_settings set port = $2 where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(port))
        .execute(pool)
        .await
        .unwrap();
    (guacd, bench)
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
