//! The terminal over real sockets (K4 Section 7): the hub's `/terminal` route on loopback,
//! the device's SSH server (a `russh` server in the test) on loopback, and a WebSocket client
//! with the subprotocol `anima-terminal`. In-crate tests, so the waits stay crate-private.

mod bench;
mod causes;
mod channels;
mod deny_list;
mod fake_ssh;
mod interruptions;
mod opening;
mod protocol;
mod refusals;
mod session;
mod socket;
mod stalls;
mod upgrade;

use std::time::Duration;

use sqlx::PgPool;

use bench::{Bench, Setup, bench};
use fake_ssh::Heard;
use socket::Socket;

/// The longest a test waits for any one thing.
const WAIT: Duration = Duration::from_secs(5);

/// A session opened by the client at 80 × 24, past the PTY and the shell.
async fn session(pool: &PgPool, setup: Setup) -> (Bench, Socket) {
    let mut bench = bench(pool, setup).await;
    let mut client = socket::open(&bench.url).await;
    client.resize(80, 24).await;
    let pty = bench.ssh.next_after_sign_in().await;
    assert!(
        matches!(
            pty,
            Heard::Pty {
                cols: 80,
                rows: 24,
                ..
            }
        ),
        "{pty:?}"
    );
    assert_eq!(bench.ssh.next().await, Heard::Shell);
    (bench, client)
}
