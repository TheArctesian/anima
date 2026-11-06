//! Opening a link (P2-003, P2-004): the accepted host key, the host resolved once and each
//! address checked, a connection to a checked address, the key pinned in the key exchange
//! (SEC-007) and the sign-in with the stored credential (OD-9). I/O edge; nothing here logs.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use russh::client::{self, Handle};
use sqlx::PgPool;
use tokio::{net::TcpStream, time::timeout};

use super::Error;
use crate::{
    credentials,
    devices::Device,
    sessions::{
        self,
        ssh::{self, Observer, Target},
        target::TargetRule,
    },
    targets::AddressPolicy,
};

/// The longest a link takes to open, from the host-key read to the end of the sign-in.
const OPENING: Duration = Duration::from_secs(10);
/// Keep-alive interval; [`KEEPALIVE_MAX`] unanswered end the connection.
const KEEPALIVE: Duration = Duration::from_secs(10);
const KEEPALIVE_MAX: usize = 3;
/// A link with no traffic but keep-alives for this long ends itself, so a caller that keeps
/// one too long does not hold the device's connection open.
const IDLE: Duration = Duration::from_secs(60);
/// Messages from the device held per channel before russh stops reading the connection, so
/// a device that floods a channel stalls instead of growing a buffer. Each is at most one SSH
/// packet (32 KiB by default).
const CHANNEL_BUFFER: usize = 8;

/// The target rule the resolved addresses must also pass. Release builds use
/// [`TargetRule::STRICT`]. This crate's tests use the loopback rule, so that a test policy
/// from `AddressPolicy::allowing_loopback` can reach the test server; the policy refuses every
/// address the strict rule refuses (tested in `link::tests::addresses`), so the policy alone
/// still refuses loopback in a test that does not allow it.
#[cfg(not(test))]
const RULE: TargetRule = TargetRule::STRICT;
#[cfg(test)]
const RULE: TargetRule = TargetRule::LOOPBACK_FOR_TESTS;

/// Opens a signed-in connection to `device` within [`OPENING`].
///
/// # Errors
/// [`Error::Timeout`] when [`OPENING`] passes first; otherwise as [`opened`].
pub(super) async fn open(
    db: &PgPool,
    vault: &credentials::Vault,
    policy: &AddressPolicy,
    device: &Device,
) -> Result<Handle<Observer>, Error> {
    timeout(OPENING, opened(db, vault, policy, device))
        .await
        .map_err(|_| Error::Timeout)?
}

/// Each step in order: host key, addresses, connection, pin, sign-in.
async fn opened(
    db: &PgPool,
    vault: &credentials::Vault,
    policy: &AddressPolicy,
    device: &Device,
) -> Result<Handle<Observer>, Error> {
    if !device.ssh.enabled {
        return Err(Error::SshOff);
    }
    let accepted = sessions::host_key(db, device.id)
        .await
        .map_err(Error::Sessions)?
        .ok_or(Error::NoHostKey)?
        .public_key;
    let stream = reach(policy, device).await?;
    let (observer, seen) = Observer::expecting(Some(accepted.clone()));
    let connected = client::connect_stream(config(), stream, observer).await;
    let presented = seen.await.map_err(|_| Error::NoKeyExchange)?;
    if presented.as_ref() != Some(&accepted) {
        return Err(Error::HostKeyChanged);
    }
    let mut handle = connected.map_err(|_| Error::NoKeyExchange)?;
    sign_in(db, vault, device, &mut handle).await?;
    Ok(handle)
}

/// Resolves the device's host once and connects to the first checked address that answers.
///
/// # Errors
/// [`Error::Unresolved`], [`Error::RefusedTarget`] when any address fails [`permitted`], and
/// [`Error::Unreachable`] when no address answers.
async fn reach(policy: &AddressPolicy, device: &Device) -> Result<TcpStream, Error> {
    let target = Target {
        host: &device.host,
        port: device.ssh.port,
        rule: RULE,
    };
    let addresses = ssh::addresses(target).await?;
    let addresses = permitted(policy, addresses).ok_or(Error::RefusedTarget)?;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect(address).await {
            return Ok(stream);
        }
    }
    Err(Error::Unreachable)
}

/// The addresses, when `policy` permits every one; `None` when it refuses any, so a name that
/// also points somewhere refused is refused whichever address would answer first (D-1).
pub(super) fn permitted(
    policy: &AddressPolicy,
    addresses: Vec<SocketAddr>,
) -> Option<Vec<SocketAddr>> {
    let refused = addresses
        .iter()
        .any(|address| !policy.permits(address.ip()));
    (!refused).then_some(addresses)
}

/// Signs in as the device's user with its stored credential. The decrypted secret is
/// dropped, its bytes zeroed, before this returns.
async fn sign_in(
    db: &PgPool,
    vault: &credentials::Vault,
    device: &Device,
    handle: &mut Handle<Observer>,
) -> Result<(), Error> {
    let secret = credentials::load_ssh(db, vault, device.id)
        .await
        .map_err(Error::Credentials)?
        .ok_or(Error::NoCredential)?;
    let signed_in = ssh::authenticate(handle, &device.ssh.username, &secret).await;
    drop(secret);
    signed_in.map_err(Error::from)
}

/// The link's SSH client configuration: the hub's host key order, keep-alives, an idle end
/// and a small channel buffer for backpressure.
fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        preferred: ssh::preferred(),
        keepalive_interval: Some(KEEPALIVE),
        keepalive_max: KEEPALIVE_MAX,
        inactivity_timeout: Some(IDLE),
        channel_buffer_size: CHANNEL_BUFFER,
        ..client::Config::default()
    })
}
