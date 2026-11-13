//! Each refusal (RA-031, VPN-006, D-4, SEC-006, SEC-007): before the relay is reached when the
//! hub's records already refuse, and through the link's own refusals; each with one Error
//! event and a fixed sentence.

use sqlx::PgPool;

use super::{
    MAC, PVE, assert_quiet, assert_stored_quiet, events, exiting, one_error_event, pve, refused,
    relay, set_mac, set_relay, wake, woken,
};
use crate::{
    fleet::{
        events::{ActorKind, Bus},
        sessions::{
            WakeError,
            tests::fixture::{DOCKER_HOST, docker_host, now},
        },
        ssh::{
            LinkError,
            tests::{SEED, bench_accepting, host_key, serve},
        },
        targets::AddressPolicy,
    },
    ids::DeviceId,
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_a_device_without_a_mac_address_is_refused_before_the_relay(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_mac(&pool, PVE, None).await;
    assert!(matches!(refused(&bench).await, WakeError::NoMacAddress));
    assert_eq!(bench.server.connections(), 0);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: the device has no MAC address. Add it in the device's \
         remote access settings.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn vpn_006_a_site_without_a_wake_relay_is_refused(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_relay(&pool, None).await;
    assert!(matches!(refused(&bench).await, WakeError::NoRelay));
    assert_eq!(bench.server.connections(), 0);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: the device's site has no wake relay. Choose one in the \
         site's settings.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_4_the_relay_does_not_wake_itself(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    set_mac(&pool, DOCKER_HOST, Some(MAC)).await;
    let error = woken(&bench, docker_host()).await.unwrap_err();
    assert!(matches!(error, WakeError::RelayIsDevice));
    assert_eq!(bench.server.connections(), 0);
    let (severity, _, device, _) = &events(&pool).await[0];
    assert_eq!(
        (severity.as_str(), device.as_deref()),
        ("error", Some("docker-host"))
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_031_an_unknown_device_is_not_found_and_writes_no_event(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let missing: DeviceId = "0190a000-0000-7000-8002-0000000000ff".parse().unwrap();
    let error = woken(&bench, missing).await.unwrap_err();
    assert!(matches!(error, WakeError::NotFound(id) if id == missing));
    assert_eq!(events(&pool).await, []);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn d_4_a_relay_with_ssh_off_is_refused_before_connecting(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    sqlx::query("update devices.ssh_settings set enabled = false where device_id = $1::uuid")
        .bind(DOCKER_HOST)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        refused(&bench).await,
        WakeError::Link(LinkError::SshOff)
    ));
    assert_eq!(bench.server.connections(), 0);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: SSH is turned off for the wake relay. Turn it on in \
         the relay's SSH settings.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_006_a_relay_without_an_accepted_key_is_refused_before_connecting(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    sqlx::query("delete from trust.host_keys")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        refused(&bench).await,
        WakeError::Link(LinkError::NoHostKey)
    ));
    assert_eq!(bench.server.connections(), 0);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: no host key is accepted for the wake relay. Test its \
         connection and accept its key.",
    )
    .await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn p2_004_a_relay_on_a_denied_address_is_refused_before_connecting(pool: PgPool) {
    let bench = relay(&pool, exiting(0)).await;
    let denied = AddressPolicy::new(vec!["127.0.0.0/8".parse().unwrap()]).allowing_loopback();
    let stores = (&bench.db, &bench.vault, &denied, &Bus::new(16));
    let error = wake(stores, pve(), ActorKind::User, now())
        .await
        .unwrap_err();
    assert!(matches!(error, WakeError::Link(LinkError::RefusedTarget)));
    assert_quiet(&format!("{error:?}"), &bench.server);
    assert_eq!(bench.server.connections(), 0);
    one_error_event(
        &pool,
        "No Wake-on-LAN packet was sent: the wake relay's host does not resolve to an address \
         the hub may connect to.",
    )
    .await;
    assert_stored_quiet(&bench).await;
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_a_relay_with_another_key_gets_no_credential_and_no_command(pool: PgPool) {
    let server = serve(SEED, exiting(0)).await;
    let bench = bench_accepting(&pool, server, host_key(SEED + 1)).await;
    set_mac(&pool, PVE, Some(MAC)).await;
    set_relay(&pool, Some(DOCKER_HOST)).await;
    assert!(matches!(
        refused(&bench).await,
        WakeError::Link(LinkError::HostKeyChanged)
    ));
    assert_eq!(bench.server.heard(), [], "no sign-in, no channel");
    assert_stored_quiet(&bench).await;
    let (severity, _, _, _) = &events(&pool).await[0];
    assert_eq!(severity, "error");
}
