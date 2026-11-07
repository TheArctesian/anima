//! What the relay hears and what a wake leaves (RA-031, D-4, CHK-012, LOG-005): one `exec` of
//! `wakeonlan` with the device's MAC address, exit status 0 as the only success, and one
//! event.

use sqlx::PgPool;

use super::{
    Answer, Heard, MAC, assert_stored_quiet, events, execs, exiting, one_error_event, pve, refused,
    relay, woken,
};
use crate::sessions::{
    LinkError, WakeError,
    tests::fixture::{docker_host, now},
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
