//! The tunnel over real sockets (K4 Section 7, WP-10 T5): a fake guacd on loopback, the hub's
//! `/tunnel` route on loopback, and a WebSocket client with the subprotocol `guacamole`.
//! These are in-crate tests so the waits and the host-key reader stay crate-private seams.

mod bench;
mod causes;
mod fake_guacd;
mod interruption;
mod refusals;
mod relaying;
mod socket;

use std::time::Duration;

use guac_protocol::{Decoder, Instruction};
use sqlx::PgPool;
use tokio::time::timeout;
use uuid::Uuid;

use bench::{Bench, Ended, Setup, bench};
use fake_guacd::{ARGS_1_6_0, CONNECTION_ID, FakeGuacd, Script, fake_guacd};
use socket::Socket;

/// The one instruction in `wire`.
fn decoded(wire: &str) -> Instruction {
    let mut instructions = Decoder::new().feed(wire.as_bytes()).unwrap();
    assert_eq!(
        instructions.len(),
        1,
        "one instruction per message: {wire:?}"
    );
    instructions.remove(0)
}

/// The next instruction the hub relayed to guacd.
async fn heard(guacd: &mut FakeGuacd) -> Instruction {
    timeout(Duration::from_secs(5), guacd.from_hub.recv())
        .await
        .expect("guacd hears from the hub")
        .expect("guacd's connection is open")
}

/// Reads the open instruction, which comes first: an empty opcode and a new UUID that is not
/// guacd's connection ID (K4 Section 4, GW-014).
async fn opened(client: &mut Socket) {
    let open = decoded(&client.text().await);
    assert_eq!(open.opcode, "");
    assert_eq!(open.args.len(), 1);
    assert!(Uuid::parse_str(&open.args[0]).is_ok(), "{:?}", open.args);
    assert_ne!(open.args[0], CONNECTION_ID);
}

/// A guacd 1.6.0 session opened by the client, past the open instruction.
async fn session(pool: &PgPool, setup: Setup) -> (FakeGuacd, Bench, Socket) {
    let script = Script::Serve {
        args: ARGS_1_6_0,
        with_ready: "",
    };
    let guacd = fake_guacd(script).await;
    let bench = bench(pool, &guacd.address, setup).await;
    let mut client = socket::open(&bench.url).await;
    opened(&mut client).await;
    (guacd, bench, client)
}

/// The end row's reason and its one audit entry (by `client`) and one event (`severity`).
fn ended_once(ended: &Ended, reason: &str, severity: &str, client: &str) {
    assert_eq!(ended.reason, reason);
    let audit = (
        "operator".to_owned(),
        client.to_owned(),
        serde_json::json!({ "reason": reason }),
    );
    assert_eq!(ended.audits, [audit], "one session.end entry");
    assert_eq!(ended.events.len(), 1, "one event: {:?}", ended.events);
    let (got_severity, source, device, _) = &ended.events[0];
    assert_eq!(
        (got_severity.as_str(), source.as_str()),
        (severity, "session")
    );
    assert_eq!(device.as_deref(), Some("docker-host"));
}
