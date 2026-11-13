//! The one SSH path for work the hub runs on a device (P2-003): metrics, checks, Wake-on-LAN
//! and files. A link resolves the device's host once, refuses the host when any address fails
//! the deny list or the target rule (P2-004, R-032), pins the accepted host key in the key
//! exchange (SEC-006, SEC-007), signs in with the stored credential, and then runs only
//! [`FixedCommand`]s (CHK-012, MET-002).
//!
//! Nothing here logs: callers write the events. No error holds a host name, an address, the
//! device's text, a command's output or a secret (LOG-007).

mod command;
mod connect;
mod run;
mod subsystem;
#[cfg(test)]
pub(crate) mod tests;

use std::time::Duration;

use chrono::{DateTime, Utc};
use russh::client::Handle;
use sqlx::PgPool;

use super::{Observer, SshError};
use crate::fleet::{credentials, devices::Device, targets::AddressPolicy, trust};

pub use command::FixedCommand;

/// The most standard output a [`SshLink::run`] keeps: 4 MiB. Past it the hub stops reading,
/// closes the command's channel and returns what it has, marked [`CommandOutput::cut`].
const OUTPUT_CAP: usize = 4 * 1024 * 1024;

/// A signed-in SSH connection pinned to the accepted host key, to a target that passed the
/// target rule and deny list (P2-004). Dropping it ends the connection.
pub struct SshLink {
    handle: Handle<Observer>,
}

/// What one command left: its exit status, its standard output up to [`OUTPUT_CAP`], and
/// whether the output was cut there. Standard error is never read.
#[derive(Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// `None` when the device sent no status: the command ended by a signal, or the hub
    /// stopped reading at the cap.
    pub exit_status: Option<u32>,
    pub stdout: Vec<u8>,
    /// The output reached [`OUTPUT_CAP`] and the rest was not read.
    pub cut: bool,
}

/// Shows the size of the output, never its bytes, so a logged value holds no device text
/// (LOG-007).
impl std::fmt::Debug for CommandOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandOutput")
            .field("exit_status", &self.exit_status)
            .field("stdout_bytes", &self.stdout.len())
            .field("cut", &self.cut)
            .finish()
    }
}

/// Why a link did not open or a command did not run. No variant holds a host name, an
/// address, the device's text or a secret; the store errors stay generic (API-010, LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SSH is turned off for the device")]
    SshOff,
    #[error("no host key is accepted for the device")]
    NoHostKey,
    #[error("the device host does not resolve")]
    Unresolved,
    #[error("the device host resolves to an address the hub may not reach")]
    RefusedTarget,
    #[error("the device's SSH port cannot be reached")]
    Unreachable,
    #[error("the device did not open the SSH link in time")]
    Timeout,
    #[error("the device did not complete an SSH key exchange")]
    NoKeyExchange,
    /// Another key than the accepted one, a certificate or a key the hub cannot read; no
    /// credential was loaded or sent (SEC-007).
    #[error("the device presented another host key than the accepted one")]
    HostKeyChanged,
    #[error("no SSH credential is stored for the device")]
    NoCredential,
    /// The stored credential was entered for another host than the device's host now; nothing
    /// was sent (SEC-011).
    #[error("the device's host changed after its SSH credential was entered")]
    CredentialForOtherHost,
    #[error("the device refused the stored SSH credential")]
    CredentialRefused,
    #[error("the stored private key or its passphrase cannot be read")]
    UnreadableKey,
    #[error("the SSH connection to the device failed")]
    Lost,
    #[error("the device refused to run the command")]
    CommandRefused,
    #[error("the command did not finish within its time limit")]
    TimeLimit,
    /// The text stays as it was when the hub's session code held this error (API-010).
    #[error("session store error")]
    Trust(#[source] trust::Error),
    #[error("credential store error")]
    Credentials(#[source] credentials::Error),
}

impl From<SshError> for Error {
    fn from(error: SshError) -> Self {
        match error {
            SshError::Unresolved => Self::Unresolved,
            SshError::RefusedTarget => Self::RefusedTarget,
            SshError::Unreachable => Self::Unreachable,
            SshError::Timeout => Self::Timeout,
            SshError::Protocol => Self::Lost,
            SshError::UnreadableKey => Self::UnreadableKey,
            SshError::AuthFailed => Self::CredentialRefused,
        }
    }
}

/// Opens a link to `device` within 10 seconds: its accepted host key, its host resolved once
/// and every address checked against `policy`, a connection to a checked address, the
/// key pinned, and the sign-in with the stored credential. The credential is loaded only
/// after the presented key equals the accepted one, and dropped as soon as the sign-in
/// returns (SEC-007). Safe to drop at any await.
///
/// `_now` is the caller's clock reading. The link takes no decision on time: an accepted
/// key has no age (SEC-006). It stays in the signature so callers pass one time through a
/// collection (K3).
///
/// # Errors
/// [`Error`] for the first step that fails.
#[expect(
    clippy::too_many_arguments,
    reason = "the hub contract (K3) fixes the signature of link"
)]
pub async fn link(
    db: &PgPool,
    vault: &credentials::Vault,
    policy: &AddressPolicy,
    device: &Device,
    _now: DateTime<Utc>,
) -> Result<SshLink, Error> {
    let handle = connect::open(db, vault, policy, device).await?;
    Ok(SshLink { handle })
}

impl SshLink {
    /// Runs `command` on its own session channel with one `exec` and no other request, within
    /// `limit`: standard output up to [`OUTPUT_CAP`], standard error dropped unread, and the
    /// exit status. The link stays open for the next command.
    ///
    /// # Errors
    /// [`Error::TimeLimit`] when `limit` passes first, [`Error::CommandRefused`] when the
    /// device refuses the channel or the `exec`, [`Error::Lost`] when the connection fails.
    pub async fn run(
        &mut self,
        command: &FixedCommand,
        limit: Duration,
    ) -> Result<CommandOutput, Error> {
        run::run(&self.handle, &command.line(), limit).await
    }
}
