//! One full session, and the relay rules on the wire: the open instruction first, guacd's
//! instructions sent with `ready`, ping, `size`, `nop` after silence, the message size cap.

use std::time::Duration;

use guac_protocol::Instruction;
use serde_json::json;
use sqlx::PgPool;

use super::{
    bench::{PASSWORD, Setup, bench, ended, stored_text},
    fake_guacd::{ARGS_1_5_5, ARGS_1_6_0, Act, CONNECTION_ID, Script, connect_values, fake_guacd},
    heard, opened,
    socket::{Received, open},
};
use crate::sessions::tests::ED25519;

const OPCODES: [&str; 7] = [
    "size", "audio", "video", "image", "timezone", "name", "connect",
];

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_010_gw_011_gw_014_ra_008_one_full_ssh_session(pool: PgPool) {
    let script = Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "4.sync,1.7;",
    };
    let mut guacd = fake_guacd(script).await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    assert_eq!(client.text().await, "4.sync,1.7;", "sent with ready (H-5)");

    // The handshake: the display, zone and operator from the ticket; connect values from the
    // database and the credential store, with the checked address as hostname (R-032).
    let handshake = (&mut guacd.handshake).await.unwrap();
    let opcodes: Vec<&str> = handshake.iter().map(|i| i.opcode.as_str()).collect();
    assert_eq!(opcodes, OPCODES);
    assert_eq!(handshake[0].args, ["1280", "800", "96"]);
    assert_eq!(handshake[4].args, ["Europe/Berlin"]);
    assert_eq!(handshake[5].args, ["operator"]);
    let values = connect_values(ARGS_1_6_0, &handshake[6]);
    assert_eq!(values["hostname"], "10.10.0.21");
    assert_eq!(values["port"], "22");
    assert_eq!(values["username"], "operator");
    assert_eq!(values["password"], PASSWORD);
    assert_eq!(
        values["host-key"],
        format!("10.10.0.21 ssh-ed25519 {ED25519}")
    );
    assert_eq!(values["private-key"], "");

    // Input reaches guacd; the internal ping comes back and does not.
    client.send("3.key,5.65307,1.1;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("key", ["65307", "1"])
    );
    let ping = "0.,4.ping,13.1730710400000;";
    client.send(ping).await;
    assert_eq!(client.text().await, ping);
    client.send("4.size,4.1024,3.768;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("size", ["1024", "768"])
    );

    // guacd's output reaches the client unchanged, one instruction per message.
    guacd
        .act
        .send(Act::Send("4.sync,4.1234;5.mouse,1.0,1.0;".to_owned()))
        .ok();
    assert_eq!(client.text().await, "4.sync,4.1234;");
    assert_eq!(client.text().await, "5.mouse,1.0,1.0;");

    assert_eq!(client.close().await, Some(1000));
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "operator");
    let audit = (
        "operator".to_owned(),
        "127.0.0.1".to_owned(),
        json!({ "reason": "operator" }),
    );
    assert_eq!(ended.audits, [audit]);
    assert_eq!(ended.events.len(), 1);
    let (severity, source, device, _) = &ended.events[0];
    assert_eq!((severity.as_str(), source.as_str()), ("info", "session"));
    assert_eq!(device.as_deref(), Some("docker-host"));

    let stored = stored_text(&pool).await;
    for secret in [CONNECTION_ID, PASSWORD, bench.token.as_str()] {
        assert!(!stored.contains(secret), "stored: {secret}");
        assert!(
            client.seen.iter().all(|text| !text.contains(secret)),
            "sent: {secret}"
        );
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_010_guacd_1_5_5_args_get_one_value_each(pool: PgPool) {
    let script = Script::Serve {
        args: ARGS_1_5_5,
        with_ready: "",
    };
    let guacd = fake_guacd(script).await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let handshake = guacd.handshake.await.unwrap();
    let values = connect_values(ARGS_1_5_5, &handshake[6]);
    assert_eq!(values["hostname"], "10.10.0.21");
    assert_eq!(values["password"], PASSWORD);
    assert_eq!(
        values["timezone"], "",
        "the timezone instruction sets it (H-9)"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_4_the_client_gets_a_nop_after_gateway_silence(pool: PgPool) {
    let script = Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    };
    let guacd = fake_guacd(script).await;
    let setup = Setup::route(|route| {
        route.timing.gateway_silence = Duration::from_millis(150);
    });
    let bench = bench(&pool, &guacd.address, setup).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    assert_eq!(client.text().await, "3.nop;");
    assert_eq!(
        client.text().await,
        "3.nop;",
        "and again after more silence"
    );
    drop(guacd);
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn gw_011_r_018_an_out_of_bounds_size_is_dropped_not_forwarded(pool: PgPool) {
    let script = Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    };
    let mut guacd = fake_guacd(script).await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    client.send("4.size,4.9000,3.768;").await;
    client.send("4.size,3.800,3.600,2.47;").await;
    client.send("4.size,3.800,3.600,2.48;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("size", ["800", "600", "48"])
    );
    client.send("4.size,1.1,4.8192;").await;
    assert_eq!(
        heard(&mut guacd).await,
        Instruction::new("size", ["1", "8192"])
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn h_3_a_message_over_64_kib_is_refused_and_never_reaches_guacd(pool: PgPool) {
    let script = Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    };
    let mut guacd = fake_guacd(script).await;
    let bench = bench(&pool, &guacd.address, Setup::default()).await;
    let mut client = open(&bench.url).await;
    opened(&mut client).await;
    let key = "3.key,5.65307,1.1;";
    let oversize = key.repeat(64 * 1024 / key.len() + 1);
    client.send(&oversize).await;
    assert!(matches!(client.next().await, Received::Closed(_)));
    assert_eq!(
        heard(&mut guacd).await.opcode,
        "disconnect",
        "no key got through"
    );
    // axum reports the refused message as a broken socket, as it does a lost client.
    let ended = ended(&pool, bench.session).await;
    assert_eq!(ended.reason, "operator");
    assert_eq!((ended.audits.len(), ended.events.len()), (1, 1));
}
