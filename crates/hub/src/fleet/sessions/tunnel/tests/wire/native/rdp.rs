//! An RDP device behind the native gateway (plan item S2, task R6): the hub reads the
//! device's certificate, sends the `connect` values of K4 Section 3, and relays the
//! session; a device with another certificate than the accepted one never gets the
//! account (SEC-007).

use std::time::Duration;

use rdp_test_device::{Device, Offered, Seen, Setup as DeviceSetup};
use sqlx::PgPool;

use super::super::{
    Bench, Setup, bench, decoded, opened,
    records::{DOCKER_HOST, LOOPBACK, ended, ended_once},
    socket::{Socket, open},
};
use super::native_gateway;
use anima_gateway::rdp::KeyEvent;

const SIZE: (u16, u16) = (640, 480);

fn rdp_device() -> Device {
    Device::start(DeviceSetup {
        size: SIZE,
        offered: Offered::Tls,
    })
}

/// The fingerprint as the hub stores an accepted certificate: pairs of hex digits.
fn stored(fingerprint: [u8; 32]) -> String {
    let pairs: Vec<_> = fingerprint
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    pairs.join(":")
}

/// An RDP session of `docker-host` through the native gateway, started, with the device's
/// RDP port set to `device` and `accepted` as its accepted certificate.
async fn rdp_bench(pool: &PgPool, device: &Device, accepted: &str) -> Bench {
    let gateway = native_gateway().await;
    let bench = bench(pool, &gateway, Setup::default()).await;
    sqlx::query(
        "update devices.rdp_settings set port = $2, security = 'tls' \
         where device_id = $1::text::uuid",
    )
    .bind(DOCKER_HOST)
    .bind(i32::from(device.address.port()))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "update trust.server_certificates set fingerprint_sha256 = $2 \
         where device_id = $1::text::uuid",
    )
    .bind(DOCKER_HOST)
    .bind(accepted)
    .execute(pool)
    .await
    .unwrap();
    bench
}

/// The client's instructions up to and with the first `sync`.
async fn first_frame(client: &mut Socket) -> (Vec<guac_protocol::Instruction>, String) {
    let mut frame = Vec::new();
    loop {
        let text = client.text().await;
        let instruction = decoded(&text);
        let sync = instruction.opcode == "sync";
        frame.push(instruction);
        if sync {
            return (frame, text);
        }
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../../tests/fixtures", scripts("devices"))
)]
async fn s2_an_rdp_session_through_the_native_gateway_signs_in_draws_and_takes_keys(pool: PgPool) {
    let mut device = rdp_device();
    let accepted = stored(device.fingerprint);
    let bench = rdp_bench(&pool, &device, &accepted).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;

    // The hub's read of the certificate ends first. Then the stored account reaches the
    // device, after the gateway saw the accepted certificate too.
    assert_eq!(device.next().await, Seen::Ended);
    let Seen::SignIn {
        username, domain, ..
    } = device.next().await
    else {
        panic!("the sign-in comes first")
    };
    assert_eq!((username.as_str(), domain.as_deref()), ("ops", Some("LAB")));

    let (frame, sync) = first_frame(&mut client).await;
    assert_eq!(frame[0].opcode, "size");
    assert_eq!(frame[0].args, ["0", "640", "480"]);
    client.send(&sync).await;

    // Num Lock, which the gateway sets once, and then the client's Return.
    assert_eq!(device.next().await, Seen::Locks(0x02));
    client.send("3.key,5.65293,1.1;").await;
    let pressed = KeyEvent::Scancode {
        code: 0x1c,
        extended: false,
        pressed: true,
    };
    assert_eq!(device.next().await, Seen::Key(pressed));

    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../../tests/fixtures", scripts("devices"))
)]
async fn sec_007_an_rdp_device_with_another_certificate_never_gets_the_account(pool: PgPool) {
    let mut device = rdp_device();
    let other = stored([7; 32]);
    let bench = rdp_bench(&pool, &device, &other).await;
    let mut client = open(&bench.url).await;

    let error = decoded(&client.text().await);
    assert_eq!(error.args, ["Certificate changed", "771"]);
    assert_eq!(client.close_code().await, Some(4003));
    ended_once(
        &ended(&pool, bench.session).await,
        "server_certificate_mismatch",
        "error",
        LOOPBACK,
    );
    // The hub's read of the certificate, and nothing after it.
    assert_eq!(device.next().await, Seen::Ended);
    let more = tokio::time::timeout(Duration::from_millis(300), device.next()).await;
    assert!(more.is_err(), "no sign-in reached the device");
}
