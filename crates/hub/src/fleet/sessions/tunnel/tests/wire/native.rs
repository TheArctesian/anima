//! The tunnel against the native gateway (`anima-gateway`) in place of guacd, and a VNC
//! device behind it (plan item S2, K4 Sections 3 to 5): the hub's handshake, relay and end
//! reasons are the same code that talks to guacd, so the pair is proved together here.
//! `rdp` does the same with an RDP device.

mod rdp;
mod vnc_device;

use std::time::Duration;

use anima_gateway::session::{Settings, serve};
use sqlx::PgPool;
use tokio::{net::TcpListener, time::timeout};

use super::{
    Bench, Setup, bench, decoded, opened,
    records::{DOCKER_HOST, LOOPBACK, ended, ended_once},
    socket::{Socket, open},
};
use crate::fleet::sessions::Protocol;
use vnc_device::{HEIGHT, VncDevice, WIDTH, vnc_device};

/// Starts the gateway on a loopback port, one session for each connection, as the program
/// does with one worker process for each.
async fn native_gateway() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        while let Ok((hub, _)) = listener.accept().await {
            tokio::spawn(
                async move { serve(hub, "$native-connection-id", &Settings::default()).await },
            );
        }
    });
    address
}

/// A VNC session of `docker-host` through the native gateway to `device`, with the stored
/// VNC password and no account name, past the open instruction.
async fn vnc_session(pool: &PgPool, device: &VncDevice) -> (Bench, Socket) {
    let gateway = native_gateway().await;
    let bench = bench(pool, &gateway, Setup::protocol(Protocol::Vnc)).await;
    sqlx::query("update devices.vnc_settings set port = $2, username = null where device_id = $1::text::uuid")
        .bind(DOCKER_HOST)
        .bind(i32::from(device.address.port()))
        .execute(pool)
        .await
        .unwrap();
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    (bench, client)
}

/// The opcodes the client gets up to and with the first `sync`, and that `sync`.
async fn first_frame(client: &mut Socket) -> (Vec<String>, String) {
    let mut opcodes = Vec::new();
    loop {
        let text = client.text().await;
        let instruction = decoded(&text);
        opcodes.push(instruction.opcode.clone());
        if instruction.opcode == "sync" {
            return (opcodes, text);
        }
        if instruction.opcode == "size" {
            assert_eq!(
                instruction.args,
                ["0", &WIDTH.to_string(), &HEIGHT.to_string()]
            );
        }
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn s2_a_vnc_session_through_the_native_gateway_draws_and_takes_keys(pool: PgPool) {
    let mut device = vnc_device(true).await;
    let (_bench, mut client) = vnc_session(&pool, &device).await;

    // The device's red screen: its size, one fill, the end of the frame.
    let (opcodes, sync) = first_frame(&mut client).await;
    assert_eq!(opcodes, ["size", "rect", "cfill", "sync"]);
    client.send(&sync).await;

    client.send("3.key,5.65293,1.1;").await;
    let key = timeout(Duration::from_secs(5), device.keys.recv())
        .await
        .unwrap();
    assert_eq!(key, Some((65293, true)));

    assert_eq!(client.close().await, Some(1000));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn s2_a_sign_in_the_device_refuses_ends_the_session_auth_failed(pool: PgPool) {
    let device = vnc_device(false).await;
    let (bench, mut client) = vnc_session(&pool, &device).await;

    // The gateway's `error` with status 0x0301, with the hub's fixed text (K4 Section 4).
    let error = decoded(&client.text().await);
    assert_eq!(error.opcode, "error");
    assert_eq!(error.args, ["Aborted. See logs.", "769"]);
    assert_eq!(client.close_code().await, Some(1011));
    ended_once(
        &ended(&pool, bench.session).await,
        "auth_failed",
        "warning",
        LOOPBACK,
    );
}
