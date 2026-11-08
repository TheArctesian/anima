//! Opening a link (P2-003, P2-004): the accepted host key, the host resolved once and each
//! address checked, a connection to a checked address, the key pinned in the key exchange
//! (SEC-007) and the sign-in with the stored credential (OD-9). I/O edge; nothing here logs.

use std::{sync::Arc, time::Duration};

use russh::client::{self, Handle};
use sqlx::PgPool;
use tokio::{net::TcpStream, time::timeout};

use super::Error;
use crate::{
    credentials,
    devices::Device,
    ssh::{self, Observer, SshError},
    targets::{self, AddressPolicy},
    trust,
};

/// The longest a link takes to open, from the host-key read to the end of the sign-in.
const OPENING: Duration = Duration::from_secs(10);
/// The longest one address may take to accept the TCP connection, so an address that drops
/// packets does not use up [`OPENING`] before the next address is tried.
const ADDRESS_WAIT: Duration = Duration::from_secs(3);
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
    let accepted = trust::host_key(db, device.id)
        .await
        .map_err(Error::Trust)?
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

/// Resolves the device's host once and connects to the first checked address that answers
/// within [`ADDRESS_WAIT`].
///
/// # Errors
/// [`Error::Unresolved`], [`Error::RefusedTarget`] when `policy` refuses any address, and
/// [`Error::Unreachable`] when no address answers.
async fn reach(policy: &AddressPolicy, device: &Device) -> Result<TcpStream, Error> {
    let addresses = targets::resolve_checked(&device.host, device.ssh.port, policy)
        .await
        .map_err(SshError::from)?;
    for address in addresses {
        if let Ok(Ok(stream)) = timeout(ADDRESS_WAIT, TcpStream::connect(address)).await {
            return Ok(stream);
        }
    }
    Err(Error::Unreachable)
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
