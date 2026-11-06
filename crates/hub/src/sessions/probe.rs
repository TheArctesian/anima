//! The SSH probe (DEV-013, K4 §6, R-034): reads a host's key with a key exchange and no
//! authentication, and signs in only when the key is the accepted one (D-5). I/O edge.
//!
//! The connection, the pinning and the sign-in are the hub's SSH client (`ssh`). Nothing here
//! logs, and no error holds a remote text, a host name or a secret (LOG-007).

use std::{sync::Arc, time::Duration};

use russh::client;

use super::{
    public_key::PublicKey,
    ssh::{self, Observer, SshError, Target},
};
use crate::credentials::SshSecret;

/// How long one probe may take, lookup, connect, key exchange and sign-in included (D-5).
/// It stays below the client's 10 s request limit (`HUB_TIMEOUT_MS`), so a device that does
/// not answer is reported as unreachable instead of the hub looking unreachable (NET-004).
pub(super) const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// What a sign-in sends, and to which key only.
pub(super) struct Login<'a> {
    pub(super) accepted: &'a PublicKey,
    pub(super) username: &'a str,
    pub(super) secret: &'a SshSecret,
}

/// What a sign-in found.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SignIn {
    /// The host presented the accepted key and took the credential.
    SignedIn,
    /// The host presented another key, so no credential was sent.
    OtherKey(PublicKey),
}

/// Reads the host key of `target`: a key exchange, then the connection closes before any
/// authentication.
///
/// # Errors
/// [`SshError`] when the host does not resolve, resolves to a refused address, cannot be
/// reached, does not finish within `timeout` or does not complete a key exchange.
pub(super) async fn read_host_key(
    target: Target<'_>,
    timeout: Duration,
) -> Result<PublicKey, SshError> {
    let probe = async {
        let (observer, seen) = Observer::expecting(None);
        let stream = ssh::connect(target).await?;
        // The observer turns down every key, so the exchange always ends here.
        let _refused = client::connect_stream(config(timeout), stream, observer).await;
        seen.await.ok().flatten().ok_or(SshError::Protocol)
    };
    tokio::time::timeout(timeout, probe)
        .await
        .map_err(|_| SshError::Timeout)?
}

/// Signs in to `target` with `login.secret` only when it presents `login.accepted`; any
/// other key ends the connection before a credential leaves the hub (D-5).
///
/// # Errors
/// As [`read_host_key`]; [`SshError::UnreadableKey`] when a private key or passphrase does
/// not decode, [`SshError::AuthFailed`] when the host refuses the credential.
pub(super) async fn sign_in(
    target: Target<'_>,
    login: &Login<'_>,
    timeout: Duration,
) -> Result<SignIn, SshError> {
    let probe = async {
        let (observer, seen) = Observer::expecting(Some(login.accepted.clone()));
        let connected =
            client::connect_stream(config(timeout), ssh::connect(target).await?, observer).await;
        let presented = seen.await.ok().flatten().ok_or(SshError::Protocol)?;
        if presented != *login.accepted {
            return Ok(SignIn::OtherKey(presented));
        }
        let mut handle = connected.map_err(|_| SshError::Protocol)?;
        let signed_in = ssh::authenticate(&mut handle, login.username, login.secret).await;
        // Closing is best effort: the result is known, and dropping the handle ends the task.
        let _closed = handle
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await;
        signed_in.map(|()| SignIn::SignedIn)
    };
    tokio::time::timeout(timeout, probe)
        .await
        .map_err(|_| SshError::Timeout)?
}

/// The client configuration: the hub's host key order, and a session that ends itself when
/// the probe is dropped mid-exchange.
fn config(timeout: Duration) -> Arc<client::Config> {
    Arc::new(client::Config {
        preferred: ssh::preferred(),
        inactivity_timeout: Some(timeout),
        ..client::Config::default()
    })
}
