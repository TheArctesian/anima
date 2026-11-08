//! Everything the terminal does before the WebSocket upgrade, after the ticket is redeemed
//! (K4 Section 8.2): load the device, resolve and check its host (R-032), connect to a
//! checked address, pin the host key in the key exchange (SEC-007, K4 Section 8.3), sign in
//! with the stored credential (OD-9, SEC-005) and open the session channel.
//!
//! The credential is sent only after the presented key equals the accepted one, and it is
//! dropped, its bytes zeroed, as soon as the sign-in returns. Nothing here logs; no error or
//! message holds a host name, an address, the device's text or a secret (LOG-007).

use std::{sync::Arc, time::Duration};

use russh::{
    Channel,
    client::{self, Handle, Msg},
};
use tokio::time::timeout;

use super::{
    ending::{NO_CREDENTIAL, UNREADABLE_KEY},
    route::Terminal,
};
use crate::{
    credentials,
    devices::{self, Device},
    sessions::{
        self, PendingTicket, Protocol,
        end::Cause,
        ssh::{self, Observer, SshError, Target},
    },
};

/// SSH keep-alive interval (K4 Section 8.2, spec Section 6.3.1); three unanswered end the
/// connection, which the relay sees as a failed connection.
const KEEPALIVE: Duration = Duration::from_secs(30);
/// Messages from the device held for the relay before russh stops reading the connection, so
/// a client that reads slowly stalls the device instead of growing a buffer (K4 Section 8.5).
/// Each is at most one SSH packet (32 KiB by default).
const CHANNEL_BUFFER: usize = 4;

/// A signed-in SSH connection and its session channel, before the PTY and shell.
pub(super) struct SignedIn {
    pub(super) handle: Handle<Observer>,
    pub(super) channel: Channel<Msg>,
}

/// What the terminal does after the upgrade.
pub(super) enum Opening {
    /// Signed in with a session channel open: wait for the client's first `resize`.
    SignedIn(SignedIn),
    /// The device presented another host key than the accepted one, or a certificate, or a
    /// key the hub cannot read: no credential was sent (SEC-007). Tell the client, close 4001.
    HostKeyChanged,
    /// No credential is stored, it cannot be read, or the device refused it: tell the client,
    /// close 1011. The message says which.
    SignInRefused(&'static str),
}

/// Why the terminal did not open. The messages are for the process log; none holds a value,
/// an address, the device's text or the token (LOG-007).
#[derive(Debug, thiserror::Error)]
pub(super) enum NotOpened {
    #[error("the device no longer exists")]
    DeviceGone,
    #[error("SSH is turned off for the device")]
    SshOff,
    #[error("no host key is accepted for the device")]
    NoHostKey,
    #[error("the device host does not resolve")]
    Unresolved,
    #[error("the device host resolves to a refused address")]
    RefusedTarget,
    #[error("the device's SSH port cannot be reached")]
    Unreachable,
    #[error("the device did not open the session in time")]
    Timeout,
    #[error("the device did not complete an SSH key exchange")]
    NoKeyExchange,
    #[error("the SSH connection failed during the sign-in or the channel open")]
    Lost,
    #[error("device store error")]
    Devices(#[source] devices::Error),
    #[error("session store error")]
    Sessions(#[source] sessions::Error),
    #[error("credential store error")]
    Credentials(#[source] credentials::Error),
}

impl NotOpened {
    /// The event message: what the operator can do about it, with no detail from the device.
    pub(super) const fn message(&self) -> &'static str {
        match self {
            Self::DeviceGone => "The device was deleted before the SSH session opened.",
            Self::SshOff => "SSH was turned off for the device before the session opened.",
            Self::NoHostKey => {
                "The device has no accepted host key. Test the connection and accept its key."
            }
            Self::Unresolved => "The device's host name does not resolve.",
            Self::RefusedTarget => {
                "The device's host resolves to a loopback, link-local, multicast or unspecified \
                 address, or to one in a network the hub refuses (ANIMA_PROBE_DENY), which the \
                 hub must not reach."
            }
            Self::Unreachable => "The hub could not reach the device's SSH port.",
            Self::Timeout => "The device did not open the SSH session in time.",
            Self::NoKeyExchange => "The device did not complete an SSH key exchange.",
            Self::Lost => "The SSH connection to the device failed while the session opened.",
            Self::Devices(_) | Self::Sessions(_) | Self::Credentials(_) => {
                "The hub could not read the session's settings from its database."
            }
        }
    }
}

impl From<SshError> for NotOpened {
    fn from(error: SshError) -> Self {
        match error {
            SshError::Unresolved => Self::Unresolved,
            SshError::RefusedTarget => Self::RefusedTarget,
            SshError::Unreachable => Self::Unreachable,
            SshError::Timeout => Self::Timeout,
            SshError::Protocol => Self::NoKeyExchange,
            SshError::UnreadableKey | SshError::AuthFailed => Self::Lost,
        }
    }
}

/// Opens the device side of the ticket's session within its opening limit
/// ([`Timing::opening_of`](super::route::Timing::opening_of)). Safe to
/// drop at any await: dropping the connection's handle ends it, and the credential leaves the
/// hub only in the sign-in, after the key matched (SEC-002).
///
/// # Errors
/// [`NotOpened`] for each step that fails; the caller ends the session `GATEWAY_ERROR`.
pub(super) async fn open(
    terminal: &Terminal,
    ticket: &PendingTicket,
) -> Result<Opening, NotOpened> {
    timeout(terminal.timing.opening_of(ticket), opened(terminal, ticket))
        .await
        .map_err(|_| NotOpened::Timeout)?
}

async fn opened(terminal: &Terminal, ticket: &PendingTicket) -> Result<Opening, NotOpened> {
    let device = devices::get(&terminal.db, ticket.device)
        .await
        .map_err(|error| match error {
            devices::Error::NotFound(_) => NotOpened::DeviceGone,
            other => NotOpened::Devices(other),
        })?;
    if !device.ssh.enabled {
        return Err(NotOpened::SshOff);
    }
    let accepted = sessions::host_key(&terminal.db, device.id)
        .await
        .map_err(NotOpened::Sessions)?
        .ok_or(NotOpened::NoHostKey)?
        .public_key;
    let target = Target {
        host: &device.host,
        port: device.ssh.port,
        policy: terminal.live.policy(),
    };
    let stream = if ticket.wake_sent {
        ssh::connect_waking(target, terminal.timing.reconnect).await?
    } else {
        ssh::connect(target).await?
    };
    let (observer, seen) = Observer::expecting(Some(accepted.clone()));
    let connected = client::connect_stream(config(), stream, observer).await;
    let presented = seen.await.map_err(|_| NotOpened::NoKeyExchange)?;
    if presented.as_ref() != Some(&accepted) {
        return Ok(Opening::HostKeyChanged);
    }
    let mut handle = connected.map_err(|_| NotOpened::NoKeyExchange)?;
    if let Some(refused) = sign_in(terminal, &device, &mut handle).await? {
        return Ok(Opening::SignInRefused(refused));
    }
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|_| NotOpened::Lost)?;
    Ok(Opening::SignedIn(SignedIn { handle, channel }))
}

/// Signs in as the device's user with its stored credential; `Some` with the event message
/// when there is none, it cannot be read or the device refuses it (D-3). The decrypted
/// secret is dropped, zeroed, before this returns.
async fn sign_in(
    terminal: &Terminal,
    device: &Device,
    handle: &mut Handle<Observer>,
) -> Result<Option<&'static str>, NotOpened> {
    let secret = credentials::load_ssh(&terminal.db, &terminal.vault, device.id)
        .await
        .map_err(NotOpened::Credentials)?;
    let Some(secret) = secret else {
        return Ok(Some(NO_CREDENTIAL));
    };
    let signed_in = ssh::authenticate(handle, &device.ssh.username, &secret).await;
    drop(secret);
    match signed_in {
        Ok(()) => Ok(None),
        Err(SshError::AuthFailed) => Ok(Some(Cause::AuthFailed.message().of(Protocol::Ssh))),
        Err(SshError::UnreadableKey) => Ok(Some(UNREADABLE_KEY)),
        Err(_) => Err(NotOpened::Lost),
    }
}

/// The terminal's SSH client configuration: the hub's host key order, keep-alive every 30
/// seconds, a small channel buffer for backpressure, and no Nagle delay for keystrokes. No
/// inactivity limit: an idle shell stays open while the keep-alive is answered.
pub(super) fn config() -> Arc<client::Config> {
    Arc::new(client::Config {
        preferred: ssh::preferred(),
        keepalive_interval: Some(KEEPALIVE),
        channel_buffer_size: CHANNEL_BUFFER,
        nodelay: true,
        ..client::Config::default()
    })
}
