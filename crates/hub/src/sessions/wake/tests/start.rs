//! A session start on a Down device (K4 Section 1, RA-031, VPN-006): the packet goes first
//! and the ticket says so, the redeemed ticket included; an Up device, one without a MAC
//! address or a site without a wake relay gets none; a wake that fails leaves the start
//! standing.

use std::sync::Arc;

use sqlx::PgPool;

use super::{Answer, MAC, PVE, events, execs, exiting, one_error_event, relay, set_mac, set_relay};
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    sessions::{
        DisplayRequest, Error, Live, Presented, Protocol, StartRequest, Ticket, Waking,
        link::tests::{Bench, reaching},
        shut_down, start,
        tests::fixture::{LOOPBACK, address, now, operator},
    },
};

/// Turns VNC on for `pve-01`, so a session needs no accepted key or certificate.
async fn vnc_on(pool: &PgPool) {
    sqlx::query("update devices.vnc_settings set enabled = true where device_id = $1::uuid")
        .bind(PVE)
        .execute(pool)
        .await
        .unwrap();
}

/// Starts a VNC session on `pve-01` from loopback on `live`, the device `asleep` or not.
async fn started(bench: &Bench, live: &Live, asleep: bool) -> Ticket {
    starting(bench, live, asleep).await.unwrap()
}

/// The start of [`started`], refused or not.
async fn starting(bench: &Bench, live: &Live, asleep: bool) -> Result<Ticket, Error> {
    vnc_on(&bench.db).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let request = StartRequest {
        device: PVE.parse().unwrap(),
        protocol: Protocol::Vnc,
        display: DisplayRequest {
            width_px: 1280,
            height_px: 800,
            dpi: 96,
            timezone: None,
        },
        wake: Some(Waking {
            asleep,
            vault: bench.vault.clone(),
            policy: reaching(),
            bus: Bus::new(16),
        }),
    };
    let client = address(LOOPBACK);
    start(&bench.db, &clock, live, &operator(), client, request).await
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_down_device_is_woken_first_and_its_ticket_says_so(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let live = Live::default();
    let ticket = started(&bench, &live, true).await;
    assert!(ticket.wake_sent);
    assert_eq!(execs(&bench.server), [format!("wakeonlan {MAC}")]);
    let presented = Presented {
        path: "/tunnel",
        ..Presented::terminal(ticket.token(), address(LOOPBACK))
    };
    let redeemed = live.redeem(presented, now()).unwrap();
    assert!(redeemed.ticket.wake_sent, "the opening waits longer");
    assert_eq!(
        redeemed.ticket.expires_at,
        now() + chrono::TimeDelta::seconds(30)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_device_that_is_not_asleep_gets_no_packet(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let ticket = started(&bench, &Live::default(), false).await;
    assert!(!ticket.wake_sent);
    assert_eq!(bench.server.connections(), 0);
    assert_eq!(events(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_down_device_without_a_mac_address_gets_no_packet(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_mac(&pool, PVE, None).await;
    let ticket = started(&bench, &Live::default(), true).await;
    assert!(!ticket.wake_sent);
    assert_eq!(bench.server.connections(), 0);
    assert_eq!(events(&pool).await, [], "no attempt, so no event");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn vpn_006_a_down_device_on_a_site_without_a_wake_relay_gets_no_packet(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_relay(&pool, None).await;
    let ticket = started(&bench, &Live::default(), true).await;
    assert!(!ticket.wake_sent);
    assert_eq!(bench.server.connections(), 0);
    assert_eq!(events(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_wake_that_fails_leaves_the_start_standing_without_the_flag(pool: PgPool) {
    let bench = relay(&pool, Answer::NoStatus).await;
    let ticket = started(&bench, &Live::default(), true).await;
    assert!(!ticket.wake_sent);
    assert_eq!(execs(&bench.server).len(), 1);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: wakeonlan failed on the wake relay. Check that it is \
         installed there.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_start_the_hub_refuses_sends_no_packet(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let live = Live::default();
    shut_down(&bench.db, &live, now()).await.unwrap();
    let refused = starting(&bench, &live, true).await;
    assert!(matches!(refused, Err(Error::Unavailable)));
    assert_eq!(bench.server.connections(), 0);
    assert_eq!(events(&pool).await, []);
}
