//! Tests of the link: the command line (`command`) and the link against a test SSH server on
//! loopback (`opening`, `running`). In-crate tests, so the loopback policy stays
//! crate-private; the address checks are the policy's (`targets::tests`). The bench and the
//! server serve the other link users' tests in `sessions` too (Wake-on-LAN).

mod command;
mod opening;
mod running;
pub(crate) mod server;

use std::time::Duration;

use sqlx::PgPool;

use server::{Answer, Server, serve};

use super::{Error, SshLink, link};
use crate::fleet::{
    credentials::Vault,
    devices::{self, Device},
    ssh::tests::fixture::{LOOPBACK, PASSWORD, docker_host, move_device, now, prepare, vault},
    targets::AddressPolicy,
};

/// The seed of the host key the test server presents.
pub(crate) const SEED: u8 = 9;
/// A command limit no healthy test command reaches.
const LIMIT: Duration = Duration::from_secs(5);

/// `docker-host` pointed at a test server, with its key accepted and its password stored.
pub(crate) struct Bench {
    pub(crate) db: PgPool,
    pub(crate) vault: Vault,
    pub(crate) device: Device,
    pub(crate) server: Server,
}

/// The only policy that reaches the loopback test server: no release build has it.
pub(crate) fn reaching() -> AddressPolicy {
    AddressPolicy::default().allowing_loopback()
}

/// A bench whose server answers `answer`.
pub(crate) async fn bench(pool: &PgPool, answer: Answer) -> Bench {
    let server = serve(SEED, answer).await;
    bench_accepting(pool, server, server::host_key(SEED)).await
}

/// A bench over `server` with `accepted` stored as the device's host key.
pub(crate) async fn bench_accepting(
    pool: &PgPool,
    server: Server,
    accepted: crate::fleet::trust::PublicKey,
) -> Bench {
    let vault = vault();
    prepare(pool, &vault, &accepted).await;
    move_device(pool, LOOPBACK, server.port).await;
    let device = devices::get(pool, docker_host()).await.unwrap();
    Bench {
        db: pool.clone(),
        vault,
        device,
        server,
    }
}

impl Bench {
    /// Opens a link to the bench's device under `policy`.
    async fn link_with(&self, policy: &AddressPolicy) -> Result<SshLink, Error> {
        link(&self.db, &self.vault, policy, &self.device, now()).await
    }

    /// Opens a link under [`reaching`], which must succeed.
    async fn opened(&self) -> SshLink {
        let Ok(link) = self.link_with(&reaching()).await else {
            panic!("the link opens");
        };
        link
    }
}

/// The error's message and debug form hold no address, host, port, device text or secret
/// (LOG-007).
fn assert_quiet(error: &Error, port: u16) {
    for text in [error.to_string(), format!("{error:?}")] {
        for secret in [
            LOOPBACK,
            "docker-host",
            "invalid",
            PASSWORD,
            &port.to_string(),
        ] {
            assert!(!text.contains(secret), "{text:?} holds {secret:?}");
        }
    }
}
