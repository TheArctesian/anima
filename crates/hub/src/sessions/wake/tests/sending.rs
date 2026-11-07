//! What the relay hears and what a wake leaves (RA-031, D-4, CHK-012, LOG-005): one `exec` of
//! `wakeonlan` with the device's MAC address, exit status 0 as the only success, and one
//! event; and no MAC address or relay output in the process log (LOG-007).

use sqlx::PgPool;

use super::{
    Answer, Heard, MAC, assert_quiet, assert_stored_quiet, events, execs, exiting, one_error_event,
    pve, refused, relay, woken,
};
use crate::{
    events::process_log,
    sessions::{
        LinkError, WakeError,
        link::tests::{SEED, server::serve},
        tests::fixture::{Capture, LOOPBACK, docker_host, move_device, now},
    },
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_the_relay_runs_wakeonlan_once_with_the_devices_mac(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let sent = woken(&bench, pve()).await.unwrap();
    assert_eq!(sent.sent_at, now());
    assert_eq!(sent.relay, docker_host());
    assert_eq!(sent.relay_name, "docker-host");
    let line = format!("wakeonlan {MAC}");
    assert_eq!(
        bench.server.requests(),
        [Heard::Auth("password"), Heard::Session, Heard::Exec(line)]
    );
    assert_eq!(bench.server.connections(), 1);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_005_a_sent_packet_writes_one_info_event_naming_the_relay(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    woken(&bench, pve()).await.unwrap();
    let info = (
        "info".to_owned(),
        "session".to_owned(),
        Some("pve-01".to_owned()),
        "Sent a Wake-on-LAN packet through the wake relay docker-host.".to_owned(),
    );
    assert_eq!(events(&pool).await, [info]);
    assert_stored_quiet(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_4_a_non_zero_exit_status_is_not_sent(pool: PgPool) {
    let bench = relay(&pool, exiting(1)).await;
    assert!(matches!(refused(&bench).await, WakeError::NotSent));
    assert_eq!(execs(&bench.server), [format!("wakeonlan {MAC}")]);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: wakeonlan failed on the wake relay. Check that it is \
         installed there.",
    )
    .await;
    assert_stored_quiet(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d_4_a_command_with_no_exit_status_is_not_sent(pool: PgPool) {
    let bench = relay(&pool, Answer::NoStatus).await;
    assert!(matches!(refused(&bench).await, WakeError::NotSent));
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
async fn d_4_a_relay_that_refuses_the_command_sends_nothing(pool: PgPool) {
    let bench = relay(&pool, Answer::Refuse).await;
    assert!(matches!(
        refused(&bench).await,
        WakeError::Link(LinkError::CommandRefused)
    ));
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: the wake relay refused to run wakeonlan.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn log_007_sec_009_no_mac_address_or_relay_output_reaches_the_process_log(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("trace", move || writer.clone()).unwrap();
    // As in the Guacamole tunnel's test: a second registered dispatcher makes every callsite
    // registration ask all of them, so another test thread cannot disable these callsites.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    tracing::info!("log_007 capture marker");

    // A failed packet with relay output on both streams, then a sent one through a relay that
    // echoes its command line, MAC address included. No event may hold a MAC address (LOG-005
    // names the relay only), so no log line may either.
    let bench = relay(&pool, exiting(1)).await;
    assert!(matches!(refused(&bench).await, WakeError::NotSent));
    let echoing = serve(SEED, Answer::Echo).await;
    move_device(&pool, LOOPBACK, echoing.port).await;
    woken(&bench, pve()).await.unwrap();
    assert_eq!(execs(&echoing), [format!("wakeonlan {MAC}")]);

    let log = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(
        log.contains("log_007 capture marker"),
        "the capture works: {log}"
    );
    assert_quiet(&log, &bench.server);
    assert_quiet(&log, &echoing);
}
