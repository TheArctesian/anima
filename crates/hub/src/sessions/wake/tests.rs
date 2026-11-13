//! Tests of Wake-on-LAN against the link's test SSH server on loopback: `docker-host` is the
//! home site's wake relay, `pve-01` the device it wakes. What the relay hears and sends
//! (`sending`), each refusal (`refusals`), the mutation with its audit entry (`mutation`),
//! and a session start that wakes the device first (`start`). In-crate, for the loopback
//! policy.

mod mutation;
mod refusals;
mod sending;
mod start;

use sqlx::PgPool;

use super::{Error, WakeSent, wake};
use crate::{
    events::{ActorKind, Bus},
    ids::DeviceId,
    sessions::tests::fixture::{DOCKER_HOST, LOOPBACK, PASSWORD, now, stored_text},
    ssh::tests::{Answer, Bench, Heard, Server, bench, reaching},
};

/// `pve-01`, on the home site, the device the tests wake.
const PVE: &str = "0190a000-0000-7000-8002-000000000001";
/// The home site of `pve-01` and of the relay `docker-host`.
const HOME: &str = "0190a000-0000-7000-8001-000000000001";
/// `pve-01`'s MAC address, as the relay must hear it.
const MAC: &str = "aa:bb:cc:dd:ee:ff";
/// The relay's own MAC address, which no packet for `pve-01` may carry.
const RELAY_MAC: &str = "02:1b:3c:4d:5e:6f";
/// Text the relay writes; nothing the hub stores or returns may hold it (LOG-007).
const OUTPUT: &str = "Sending magic packet to 255.255.255.255:9";

fn pve() -> DeviceId {
    PVE.parse().unwrap()
}

/// An answer that writes [`OUTPUT`] to both streams and exits with `status`.
fn exiting(status: u32) -> Answer {
    Answer::Exit {
        stdout: OUTPUT.as_bytes().to_vec(),
        stderr: OUTPUT.as_bytes().to_vec(),
        status,
    }
}

/// A bench whose relay answers `answer`, with `pve-01`'s and the relay's MAC addresses set
/// and `docker-host` as the home site's wake relay.
async fn relay(pool: &PgPool, answer: Answer) -> Bench {
    let bench = bench(pool, answer).await;
    set_mac(pool, PVE, Some(MAC)).await;
    set_mac(pool, DOCKER_HOST, Some(RELAY_MAC)).await;
    set_relay(pool, Some(DOCKER_HOST)).await;
    bench
}

async fn set_mac(pool: &PgPool, device: &str, mac: Option<&str>) {
    sqlx::query("update devices.devices set mac_address = $2::macaddr where id = $1::uuid")
        .bind(device)
        .bind(mac)
        .execute(pool)
        .await
        .unwrap();
}

async fn set_relay(pool: &PgPool, relay: Option<&str>) {
    sqlx::query("update devices.sites set wake_relay_device_id = $2::uuid where id = $1::uuid")
        .bind(HOME)
        .bind(relay)
        .execute(pool)
        .await
        .unwrap();
}

/// Wakes `device` under the loopback policy at [`now`].
async fn woken(bench: &Bench, device: DeviceId) -> Result<WakeSent, Error> {
    let stores = (&bench.db, &bench.vault, &reaching(), &Bus::new(16));
    wake(stores, device, ActorKind::User, now()).await
}

/// Wakes `pve-01`, which must fail; the error's message and debug form are quiet.
async fn refused(bench: &Bench) -> Error {
    match woken(bench, pve()).await {
        Ok(sent) => panic!("the packet was sent: {sent:?}"),
        Err(error) => {
            for text in [error.to_string(), format!("{error:?}")] {
                assert_quiet(&text, &bench.server);
            }
            error
        }
    }
}

/// The command lines the relay was asked to run.
fn execs(server: &Server) -> Vec<String> {
    server
        .heard()
        .into_iter()
        .filter_map(|heard| match heard {
            Heard::Exec(line) => Some(line),
            _ => None,
        })
        .collect()
}

/// `(severity, source, device name, message)` of each stored event.
async fn events(pool: &PgPool) -> Vec<(String, String, Option<String>, String)> {
    sqlx::query_as("select severity, source, device_name, message from events.event_log")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The one stored event: an Error for `pve-01` with `message`.
async fn one_error_event(pool: &PgPool, message: &str) {
    let error = (
        "error".to_owned(),
        "session".to_owned(),
        Some("pve-01".to_owned()),
        message.to_owned(),
    );
    assert_eq!(events(pool).await, [error]);
}

/// `mac`'s octets in decimal as `Debug` prints a byte array: `170, 187, 204, ...`.
fn decimal_octets(mac: &str) -> String {
    let octets: Vec<String> = mac
        .split(':')
        .map(|pair| u8::from_str_radix(pair, 16).unwrap().to_string())
        .collect();
    octets.join(", ")
}

/// `text` holds no address, port, MAC address (in hex or decimal octets), relay output or
/// secret (LOG-007).
fn assert_quiet(text: &str, server: &Server) {
    for secret in [
        LOOPBACK,
        &server.port.to_string(),
        MAC,
        RELAY_MAC,
        &decimal_octets(MAC),
        &decimal_octets(RELAY_MAC),
        OUTPUT,
        PASSWORD,
    ] {
        assert!(!text.contains(secret), "{text:?} holds {secret:?}");
    }
}

/// Everything stored about sessions, audit and events is quiet.
async fn assert_stored_quiet(bench: &Bench) {
    assert_quiet(&stored_text(&bench.db).await, &bench.server);
}
