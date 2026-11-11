//! A session start on a Down device (K4 Section 1, RA-031, VPN-006): the packet goes first
//! and the ticket says so, the redeemed ticket included; an Up device, one without a MAC
//! address or a site without a wake relay gets none; a wake that fails leaves the start
//! standing; a start refused because the hub cannot record sends none, and recording turned
//! on during the wake is checked before the row commits, and recording turned off then is followed.

use std::sync::Arc;

use sqlx::PgPool;

use super::{Answer, MAC, PVE, events, execs, exiting, one_error_event, relay, set_mac, set_relay};
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    sessions::{
        DisplayRequest, Error, Live, Presented, Protocol, Recording, StartRequest, Ticket, Waking,
        shut_down, start,
        tests::fixture::{LOOPBACK, address, no_recording, now, operator},
    },
    ssh::tests::{Bench, reaching},
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
    recorded_to(bench, live, asleep, no_recording()).await
}

/// A VNC start of `pve-01` that wakes it when `asleep` and records to `recording`.
fn request(bench: &Bench, asleep: bool, recording: Recording) -> StartRequest {
    StartRequest {
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
        recording,
    }
}

/// [`starting`] with the hub recording to `recording`.
async fn recorded_to(
    bench: &Bench,
    live: &Live,
    asleep: bool,
    recording: Recording,
) -> Result<Ticket, Error> {
    vnc_on(&bench.db).await;
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let request = request(bench, asleep, recording);
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

/// Turns recording on for `pve-01` through `db`.
async fn record_sessions(db: &PgPool) {
    set_recording(db, true).await;
}

/// Turns recording on or off for `pve-01` through `db`.
async fn set_recording(db: &PgPool, on: bool) {
    sqlx::query("update devices.devices set record_sessions = $2 where id = $1::uuid")
        .bind(PVE)
        .bind(on)
        .execute(db)
        .await
        .unwrap();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_14_a_recorded_start_the_hub_cannot_record_sends_no_packet(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    record_sessions(&bench.db).await;
    let refused = starting(&bench, &Live::default(), true).await;
    assert!(matches!(refused, Err(Error::CannotRecord)), "{refused:?}");
    assert_eq!(bench.server.connections(), 0, "no Wake-on-LAN packet");
    assert_eq!(events(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_040_recording_turned_on_during_the_wake_is_checked_before_the_row_commits(
    pool: PgPool,
) {
    let bench = relay(&pool, Answer::Hang).await;
    let db = bench.db.clone();
    let turn_on = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        record_sessions(&db).await;
    });
    let started = starting(&bench, &Live::default(), true).await;
    turn_on.await.unwrap();
    // The hub has no recording root, so a start that sees the flag is refused.
    assert!(matches!(started, Err(Error::CannotRecord)), "{started:?}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_040_recording_turned_off_during_the_wake_is_followed(pool: PgPool) {
    let bench = relay(&pool, Answer::Hang).await;
    record_sessions(&bench.db).await;
    let root = tempfile::tempdir().unwrap();
    let storage = Recording {
        root: Some(root.path().to_owned()),
        max_bytes: 1 << 30,
    };
    let db = bench.db.clone();
    let turn_off = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        set_recording(&db, false).await;
    });
    let ticket = recorded_to(&bench, &Live::default(), true, storage)
        .await
        .unwrap();
    turn_off.await.unwrap();
    assert!(
        !ticket.recording,
        "the row committed after recording was turned off"
    );
    let recorded: i64 =
        sqlx::query_scalar("select count(*) from events.audit_log where change ? 'recorded'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recorded, 0, "the audit agrees with the ticket");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_040_a_device_deleted_while_the_start_waits_on_its_row_is_not_found(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    vnc_on(&bench.db).await;
    let mut deleting = pool.begin().await.unwrap();
    sqlx::query("delete from devices.devices where id = $1::uuid")
        .bind(PVE)
        .execute(&mut *deleting)
        .await
        .unwrap();
    let db = bench.db.clone();
    let request = request(&bench, false, no_recording());
    let waiting = tokio::spawn(async move {
        let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
        let client = address(LOOPBACK);
        start(&db, &clock, &Live::default(), &operator(), client, request).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    assert!(!waiting.is_finished(), "the start waits on the device row");
    deleting.commit().await.unwrap();
    let result = waiting.await.unwrap();
    assert!(matches!(result, Err(Error::NotFound(_))), "{result:?}");
    for table in ["sessions.sessions", "events.audit_log"] {
        let rows: i64 = sqlx::query_scalar(&format!("select count(*) from {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "{table}");
    }
}
