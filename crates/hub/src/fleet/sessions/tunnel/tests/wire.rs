//! The tunnel over real sockets (K4 Section 7): a fake guacd on loopback, the hub's `/tunnel`
//! route on loopback, an RDP or VNC session on a device at a loopback address, and a
//! WebSocket client with the subprotocol `guacamole`. These are in-crate tests so the waits
//! stay crate-private and the session task's parts can be driven one at a time.

mod bench;
mod causes;
mod certificate;
mod deny_list;
mod devices;
mod fake_guacd;
mod interruption;
mod moved;
mod opening;
mod policy;
mod refusals;
mod relaying;
mod socket;
mod upgrade;

use std::time::Duration;

use guac_protocol::{Decoder, Instruction};
use sqlx::PgPool;
use tokio::time::timeout;
use uuid::Uuid;

use crate::fleet::sessions::tests::fixture as records;
use bench::{Bench, Setup, bench};
use fake_guacd::{CONNECTION_ID, FakeGuacd, RDP_ARGS, Script, fake_guacd};
use records::ended_once;
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
    assert_eq!(open.args[0].len(), 36, "K4 Section 4: `0.,36.<uuid>;`");
    assert_ne!(open.args[0], CONNECTION_ID);
}

/// A guacd 1.6.0 RDP session opened by the client, past the open instruction.
async fn session(pool: &PgPool, setup: Setup) -> (FakeGuacd, Bench, Socket) {
    let script = Script::Serve {
        args: RDP_ARGS,
        with_ready: "",
    };
    let guacd = fake_guacd(script).await;
    let bench = bench(pool, &guacd.address, setup).await;
    let mut client = socket::open(&bench.url).await;
    opened(&mut client).await;
    (guacd, bench, client)
}
