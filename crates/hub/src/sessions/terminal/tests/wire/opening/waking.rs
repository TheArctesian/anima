//! The opening of a ticket whose start sent a Wake-on-LAN packet (K4 Section 1, RA-031): it
//! waits past the usual limit, trying the device again until it answers; without the packet
//! the opening does not wait for a device that does not answer.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use super::super::{
    super::super::opening::{self, NotOpened, Opening},
    bench::{Bench, Setup, bench},
};
use crate::sessions::{
    Presented, Redeemed,
    tests::fixture::{LOOPBACK, address, move_device, now, waking_port},
};

/// The usual opening limit, shortened; the waking device answers only well after it.
const USUAL: Duration = Duration::from_millis(200);
/// When the waking device starts to answer.
const ANSWERS_AFTER: Duration = Duration::from_millis(800);

/// A bench whose opening limit is [`USUAL`], or 10 s after a wake, and whose device answers
/// only after [`ANSWERS_AFTER`], forwarding to the fake SSH server.
async fn waking(pool: &PgPool) -> Bench {
    let setup = Setup::terminal(|terminal| {
        terminal.timing.opening = USUAL;
        terminal.timing.woken_opening = Duration::from_secs(10);
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
