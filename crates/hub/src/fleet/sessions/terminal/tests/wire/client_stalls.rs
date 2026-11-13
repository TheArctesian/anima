//! A client that stops reading what the hub sends, or leaves while the send waits (K4
//! Sections 4 and 8.5, MV-003): the send stalls past the idle wait, or fails when the
//! connection is reset.

use std::{sync::Arc, time::Duration};

use sqlx::PgPool;
use tokio::{task::JoinHandle, time::sleep};

use super::{bench::Setup, fake_ssh::FakeSsh, session};
use crate::fleet::sessions::tests::fixture::{LOOPBACK, ended, ended_once};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_a_send_that_stalls_is_timeout_4002(pool: PgPool) {
    // The client reads nothing while the shell writes more than the sockets hold, so the
    // hub's send to the client stalls past the idle wait.
    let idle = Duration::from_secs(1);
    let setup = Setup::terminal(move |terminal| terminal.timing.client_idle = idle);
    let (bench, mut client) = session(&pool, setup).await;
    let flood = flood_unread(&Arc::new(bench.ssh));
    // The client reads nothing until the hub's send has stalled past the idle wait, then
    // drains while the hub's `end` and close are still on their way.
    tokio::time::sleep(idle + Duration::from_millis(500)).await;
    assert_eq!(client.ended().await, ("TIMEOUT".to_owned(), Some(4002)));
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "timeout", "info", LOOPBACK);
    assert_eq!(
        ended.events[0].3,
        "The client stopped reading what the hub sent, so the hub closed the SSH session."
    );
    flood.abort();
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn k4_8_5_mv_003_a_send_to_a_client_that_reset_the_connection_is_operator(pool: PgPool) {
    // The hub's send waits on a client that reads nothing (the idle wait is long); the client
    // then drops the connection with data unread, which resets it: the client left.
    let setup = Setup::terminal(|terminal| terminal.timing.client_idle = Duration::from_secs(30));
    let (bench, client) = session(&pool, setup).await;
    let flood = flood_unread(&Arc::new(bench.ssh));
    sleep(Duration::from_millis(700)).await;
    drop(client);
    let ended = ended(&pool, bench.session).await;
    ended_once(&ended, "operator", "info", LOOPBACK);
    flood.abort();
}

/// The shell writing more than the sockets hold, until the connection is gone.
fn flood_unread(ssh: &Arc<FakeSsh>) -> JoinHandle<()> {
    let writer = Arc::clone(ssh);
    tokio::spawn(async move {
        let chunk = vec![b'x'; 32 * 1024];
        for _ in 0..1_024 {
            if writer.try_say(&chunk).await.is_err() {
                return;
            }
        }
    })
}
