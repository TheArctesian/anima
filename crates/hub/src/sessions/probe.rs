//! The SSH probe (DEV-013, K4 §6, R-034): reads a host's key with a key exchange and no
//! authentication, and signs in only when the key is the accepted one (D-5). I/O edge.
//!
//! The host is resolved first and every address it resolves to must pass the target rule
//! (R-032); the probe then connects to a checked address, never to the name again, so a
//! second lookup cannot point it somewhere else. Nothing here logs, and no error holds a
//! remote text, a host name or a secret (LOG-007).

use std::{borrow::Cow, net::SocketAddr, sync::Arc, time::Duration};

use russh::{
    Preferred,
    client::{self, AuthResult, Handle},
    keys::{
        Algorithm, EcdsaCurve, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate,
        decode_secret_key,
    },
};
use tokio::{net::TcpStream, sync::oneshot};

use super::{public_key::PublicKey, target::TargetRule};
use crate::credentials::{Secret, SshSecret};

/// How long one probe may take, lookup, connect, key exchange and sign-in included (D-5).
pub(super) const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Host key algorithms in libssh2's order (`hostkey.c`, 1.11), because guacd sets no
/// preference: the key the probe reads is then the key guacd is pinned to (SEC-007). SHA-1
/// `ssh-rsa` is left out; an RSA key still reads as `rsa-sha2-*`, and its blob is the same.
///
/// Leaving SHA-1 out here does not stop guacd: its libssh2, and russh, may still accept a
/// device that offers only `ssh-rsa`. The hub pins whatever key the device presents and the
/// operator accepts (SEC-006); it does not judge the device's algorithms.
const HOST_KEY_ORDER: &[Algorithm] = &[
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP256,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP384,
    },
    Algorithm::Ecdsa {
        curve: EcdsaCurve::NistP521,
    },
    Algorithm::Ed25519,
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha512),
    },
    Algorithm::Rsa {
        hash: Some(HashAlg::Sha256),
    },
];

/// Why a probe gave no key or did not sign in. Holds no host name, remote text or secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ProbeError {
    #[error("the host name does not resolve")]
    Unresolved,
    #[error("the host resolves to an address sessions may not reach")]
    RefusedTarget,
    #[error("no connection to the host's SSH port")]
    Unreachable,
    #[error("the host did not finish in time")]
    Timeout,
    #[error("the host did not complete an SSH key exchange")]
    Protocol,
    #[error("the private key or its passphrase cannot be read")]
    UnreadableKey,
    #[error("the host refused the credential")]
    AuthFailed,
}

/// Where a probe goes: the device's host (a name or an IP literal) and SSH port, and the rule
/// for the addresses the host resolves to.
#[derive(Clone, Copy, Debug)]
pub(super) struct Target<'a> {
    pub(super) host: &'a str,
    pub(super) port: u16,
    pub(super) rule: TargetRule,
}

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
/// [`ProbeError`] when the host does not resolve, resolves to a refused address, cannot be
/// reached, does not finish within `timeout` or does not complete a key exchange.
pub(super) async fn read_host_key(
    target: Target<'_>,
    timeout: Duration,
) -> Result<PublicKey, ProbeError> {
    let probe = async {
        let (observer, seen) = Observer::expecting(None);
        let stream = connect(target).await?;
        // The observer turns down every key, so the exchange always ends here.
        let _refused = client::connect_stream(config(timeout), stream, observer).await;
        seen.await.map_err(|_| ProbeError::Protocol)
    };
    tokio::time::timeout(timeout, probe)
        .await
        .map_err(|_| ProbeError::Timeout)?
}

/// Signs in to `target` with `login.secret` only when it presents `login.accepted`; any
/// other key ends the connection before a credential leaves the hub (D-5).
///
/// # Errors
/// As [`read_host_key`]; [`ProbeError::UnreadableKey`] when a private key or passphrase does
/// not decode, [`ProbeError::AuthFailed`] when the host refuses the credential.
pub(super) async fn sign_in(
    target: Target<'_>,
    login: &Login<'_>,
    timeout: Duration,
) -> Result<SignIn, ProbeError> {
    let probe = async {
        let (observer, seen) = Observer::expecting(Some(login.accepted.clone()));
        let connected =
            client::connect_stream(config(timeout), connect(target).await?, observer).await;
        let presented = seen.await.map_err(|_| ProbeError::Protocol)?;
        if presented != *login.accepted {
            return Ok(SignIn::OtherKey(presented));
        }
        let mut handle = connected.map_err(|_| ProbeError::Protocol)?;
        let signed_in = authenticate(&mut handle, login).await;
        // Closing is best effort: the result is known, and dropping the handle ends the task.
        let _closed = handle
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await;
        signed_in.map(|()| SignIn::SignedIn)
    };
    tokio::time::timeout(timeout, probe)
        .await
        .map_err(|_| ProbeError::Timeout)?
}

/// The client configuration: libssh2's host key order, and a session that ends itself when
/// the probe is dropped mid-exchange.
fn config(timeout: Duration) -> Arc<client::Config> {
    Arc::new(client::Config {
        preferred: Preferred {
            key: Cow::Borrowed(HOST_KEY_ORDER),
            ..Preferred::DEFAULT
        },
        inactivity_timeout: Some(timeout),
        ..client::Config::default()
    })
}

/// Resolves the host, refuses it when any address fails the rule, and connects to the first
/// address that answers.
async fn connect(target: Target<'_>) -> Result<TcpStream, ProbeError> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((target.host, target.port))
        .await
        .map_err(|_| ProbeError::Unresolved)?
        .collect();
    if addresses.is_empty() {
        return Err(ProbeError::Unresolved);
    }
    let addresses = target
        .rule
        .checked(addresses)
        .ok_or(ProbeError::RefusedTarget)?;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect(address).await {
            return Ok(stream);
        }
    }
    Err(ProbeError::Unreachable)
}

/// Sends the credential; a refusal is [`ProbeError::AuthFailed`].
async fn authenticate(handle: &mut Handle<Observer>, login: &Login<'_>) -> Result<(), ProbeError> {
    let result = match login.secret {
        SshSecret::Password(password) => {
            let password =
                std::str::from_utf8(password.expose()).map_err(|_| ProbeError::AuthFailed)?;
            handle
                .authenticate_password(login.username, password)
                .await
                .map_err(|_| ProbeError::Protocol)?
        }
        SshSecret::PrivateKey { key, passphrase } => {
            let key = private_key(key.expose(), passphrase.as_ref().map(Secret::expose))?;
            let hash = if key.algorithm().is_rsa() {
                handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|_| ProbeError::Protocol)?
                    .flatten()
            } else {
                None
            };
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            handle
                .authenticate_publickey(login.username, key)
                .await
                .map_err(|_| ProbeError::Protocol)?
        }
    };
    match result {
        AuthResult::Success => Ok(()),
        AuthResult::Failure { .. } => Err(ProbeError::AuthFailed),
    }
}

/// Decodes an `OpenSSH`, PEM or `PuTTY` private key; the decoder's error is dropped unread.
fn private_key(
    key: &[u8],
    passphrase: Option<&[u8]>,
) -> Result<russh::keys::PrivateKey, ProbeError> {
    let key = std::str::from_utf8(key).map_err(|_| ProbeError::UnreadableKey)?;
    let passphrase = passphrase
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|_| ProbeError::UnreadableKey)?;
    decode_secret_key(key, passphrase).map_err(|_| ProbeError::UnreadableKey)
}

/// The probe's side of the key exchange: reports the presented key once, and lets the
/// connection go on only when it is the expected key.
struct Observer {
    expected: Option<PublicKey>,
    seen: Option<oneshot::Sender<PublicKey>>,
}

impl Observer {
    fn expecting(expected: Option<PublicKey>) -> (Self, oneshot::Receiver<PublicKey>) {
        let (seen, receiver) = oneshot::channel();
        let observer = Self {
            expected,
            seen: Some(seen),
        };
        (observer, receiver)
    }

    /// The key as the hub stores it, rebuilt from the bytes russh decoded (T1 note): a
    /// certificate or a key the hub does not accept counts as no key.
    fn presented(presented: &PublicKeyOrCertificate) -> Option<PublicKey> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
            return None;
        };
        PublicKey::from_blob(key.to_bytes().ok()?).ok()
    }
}

impl client::Handler for Observer {
    type Error = russh::Error;

    /// russh has verified the exchange signature with this key before it calls this.
    fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let presented = Self::presented(server_public_key);
        let go_on = presented.is_some() && presented == self.expected;
        if let (Some(key), Some(seen)) = (presented, self.seen.take()) {
            // The receiver is gone only when the probe was dropped; nobody is left to tell.
            let _gone = seen.send(key);
        }
        std::future::ready(Ok(go_on))
    }
}
