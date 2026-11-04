//! One full session, and the relay rules on the wire: the open instruction first, guacd's
//! instructions sent with `ready`, ping, `size`, `nop` after silence, the client's
//! `disconnect`, the messages the WebSocket refuses (over the size cap, not UTF-8), and
//! non-BMP text that guacamole-common-js 1.5.0 would misread (R-046).

use std::time::Duration;

use guac_protocol::{Instruction, encode};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Instant, timeout};

use super::{
    bench::{PASSWORD, Setup, bench},
    fake_guacd::{ARGS_1_5_5, ARGS_1_6_0, Act, CONNECTION_ID, Script, connect_values, fake_guacd},
    heard, opened,
    records::{ended, stored_text},
    socket::{decoded_as_utf16, open},
};

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
    let port = bench.ssh_port;
    assert_eq!(values["hostname"], "127.0.0.1");
    assert_eq!(values["port"], port.to_string());
    assert_eq!(values["username"], "operator");
    assert_eq!(values["password"], PASSWORD);
    let key = bench.host_key.to_base64();
    assert_eq!(
        values["host-key"],
        format!("[127.0.0.1]:{port} ssh-ed25519 {key}"),
        "pinned to the key the operator accepted (SEC-007)"
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
    assert_eq!(values["hostname"], "127.0.0.1");
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
async fn k4_4_the_client_gets_one_nop_per_gateway_silence(pool: PgPool) {
    let silence = Duration::from_millis(200);
    let setup = Setup::route(move |route| route.timing.gateway_silence = silence);
    let (_guacd, _bench, mut client) = super::session(&pool, setup).await;
    let started = Instant::now();
    for _ in 0..3 {
        assert_eq!(client.text().await, "3.nop;");
    }
    // Each nop restarts the wait, so three take three silences, not a burst.
    assert!(started.elapsed() >= silence * 2, "{:?}", started.elapsed());
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_4_guacd_traffic_holds_the_nop_back(pool: PgPool) {
    let setup = Setup::route(|route| route.timing.gateway_silence = Duration::from_millis(600));
    let (guacd, _bench, mut client) = super::session(&pool, setup).await;
    // Six syncs 200 ms apart span twice the silence; none of the gaps reaches it.
    for _ in 0..6 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        guacd.act.send(Act::Send("4.sync,1.1;".to_owned())).ok();
        assert_eq!(client.text().await, "4.sync,1.1;");
    }
    assert_eq!(client.text().await, "3.nop;", "until guacd goes quiet");
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
async fn h_3_r_039_a_message_over_64_kib_is_1002_protocol_error_and_never_reaches_guacd(
    pool: PgPool,
) {
    let (mut guacd, bench, mut client) = super::session(&pool, Setup::default()).await;
    let key = "3.key,5.65307,1.1;";
    let oversize = key.repeat(64 * 1024 / key.len() + 1);
    client.send(&oversize).await;
    assert_eq!(client.close_code().await, Some(1002));
    assert_eq!(
        heard(&mut guacd).await.opcode,
        "disconnect",
        "no key got through"
    );
    let ended = ended(&pool, bench.session).await;
    super::ended_once(&ended, "protocol_error", "warning", "127.0.0.1");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_039_a_text_message_that_is_not_utf_8_is_1002_protocol_error(pool: PgPool) {
    let (mut guacd, bench, mut client) = super::session(&pool, Setup::default()).await;
    client.send_unchecked_text(b"3.key,5.6530\xff,1.1;").await;
    assert_eq!(client.close_code().await, Some(1002));
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
    let ended = ended(&pool, bench.session).await;
    super::ended_once(&ended, "protocol_error", "warning", "127.0.0.1");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_5_the_clients_disconnect_reaches_guacd_once(pool: PgPool) {
    let (mut guacd, _bench, mut client) = super::session(&pool, Setup::default()).await;
    client.send("10.disconnect;").await;
    assert_eq!(heard(&mut guacd).await.opcode, "disconnect");
    let after = timeout(Duration::from_secs(5), guacd.from_hub.recv()).await;
    assert_eq!(after.unwrap(), None, "the hub closes without a second one");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
)]
async fn r_046_gw_014_r_035_a_non_bmp_title_cannot_forge_client_instructions(pool: PgPool) {
    // A terminal title the remote host chose: emoji that 1.5.0 counts twice, then a `;` and
    // the instructions 1.5.0 would read after it.
    let forged = "9.clipboard,1.0,10.text/plain;4.blob,1.0,8.SGVsbG8=;3.end,1.0;\
                  5.error,13.forged detail,3.769";
    let k = forged.chars().count() + 1;
    let title = format!("{}x;{forged}", "😀".repeat(k));
    let (guacd, _bench, mut client) = super::session(&pool, Setup::default()).await;
    let name = encode(&Instruction::new("name", [title]));
    guacd.act.send(Act::Send(name)).ok();
    let read = decoded_as_utf16(&client.text().await);
    let opcodes: Vec<&str> = read.iter().map(|i| i.opcode.as_str()).collect();
    assert_eq!(opcodes, ["name"], "nothing forged reaches the client");
    let shown = format!("{}x;{forged}", "\u{FFFD}".repeat(k));
    assert_eq!(read[0].args, [shown]);
}
