//! The hub's SSH client side, shared by the connection test and the terminal (R-034, SEC-007):
//! the host key order, the handler that pins the presented key, the checked connect (R-032)
//! and the sign-in with a stored credential (OD-9). I/O edge.
//!
//! The host is resolved first and every address it resolves to must pass the target rule
//! (R-032); the connection then goes to a checked address, never to the name again, so a
//! second lookup cannot point it somewhere else. Nothing here logs, and no error holds a
//! remote text, a host name or a secret (LOG-007).

use std::{borrow::Cow, net::SocketAddr, sync::Arc};

use russh::{
    MethodKind, Preferred,
    client::{self, AuthResult, Handle, KeyboardInteractiveAuthResponse},
    keys::{
        Algorithm, EcdsaCurve, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate,
        decode_secret_key,
    },
};
use tokio::{net::TcpStream, sync::oneshot};

use super::{public_key::PublicKey, target::TargetRule};
use crate::credentials::{Secret, SshSecret};

/// Host key algorithms in libssh2's order (`hostkey.c`, 1.11), the order guacd used, so the
/// key the connection test reads is the key a session meets (SEC-007). SHA-1 `ssh-rsa` is
/// left out; an RSA key still reads as `rsa-sha2-*`, and its blob is the same.
///
/// Leaving SHA-1 out here does not stop a device that offers only `ssh-rsa`: russh may still
/// accept it. The hub pins whatever key the device presents and the operator accepts
/// (SEC-006); it does not judge the device's algorithms.
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

/// The most keyboard-interactive rounds the hub answers before it counts the sign-in as
/// refused, so a device cannot keep it answering prompts (D-5).
const PROMPT_ROUNDS: usize = 4;

/// Why an SSH connection gave no key or did not sign in. Holds no host name, remote text or
/// secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum SshError {
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

/// Where a connection goes: the device's host (a name or an IP literal) and SSH port, and the
/// rule for the addresses the host resolves to.
#[derive(Clone, Copy, Debug)]
pub(super) struct Target<'a> {
    pub(super) host: &'a str,
    pub(super) port: u16,
    pub(super) rule: TargetRule,
}

/// The algorithm preferences every hub SSH connection uses: [`HOST_KEY_ORDER`] for the host
/// key, russh's defaults for the rest.
pub(super) fn preferred() -> Preferred {
    Preferred {
        key: Cow::Borrowed(HOST_KEY_ORDER),
        ..Preferred::DEFAULT
    }
}

/// The addresses `target`'s host resolves to, each in its canonical form (an IPv4-mapped
/// address is the IPv4 address it is), when the rule lets the hub reach every one of them; a
/// name that also points somewhere refused is refused whichever address would answer first.
///
/// # Errors
/// [`SshError::Unresolved`] when the host has no address, [`SshError::RefusedTarget`] when
/// any address fails the rule.
pub(super) async fn addresses(target: Target<'_>) -> Result<Vec<SocketAddr>, SshError> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((target.host, target.port))
        .await
        .map_err(|_| SshError::Unresolved)?
        .map(|address| SocketAddr::new(address.ip().to_canonical(), target.port))
        .collect();
    if addresses.is_empty() {
        return Err(SshError::Unresolved);
    }
    target
        .rule
        .checked(addresses)
        .ok_or(SshError::RefusedTarget)
}

/// Connects to the first of `target`'s checked [`addresses`] that answers.
///
/// # Errors
/// As [`addresses`]; [`SshError::Unreachable`] when no address answers.
pub(super) async fn connect(target: Target<'_>) -> Result<TcpStream, SshError> {
    for address in addresses(target).await? {
        if let Ok(stream) = TcpStream::connect(address).await {
            return Ok(stream);
        }
    }
    Err(SshError::Unreachable)
}

/// Signs in as `username` with `secret`: a password (answering keyboard-interactive prompts
/// with it when the device offers that and not `password`, D-5), or a private key with its
/// passphrase. russh copies what it sends into buffers of its own, which this cannot wipe.
///
/// # Errors
/// [`SshError::AuthFailed`] when the device refuses, [`SshError::UnreadableKey`] when the key
/// or passphrase does not decode, [`SshError::Protocol`] when the connection fails meanwhile.
pub(super) async fn authenticate(
    handle: &mut Handle<Observer>,
    username: &str,
    secret: &SshSecret,
) -> Result<(), SshError> {
    let result = match secret {
        SshSecret::Password(password) => {
            let password =
                std::str::from_utf8(password.expose()).map_err(|_| SshError::AuthFailed)?;
            sign_in_with_password(handle, username, password).await?
        }
        SshSecret::PrivateKey { key, passphrase } => {
            let key = private_key(key.expose(), passphrase.as_ref().map(Secret::expose))?;
            let hash = if key.algorithm().is_rsa() {
                handle
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|_| SshError::Protocol)?
                    .flatten()
            } else {
                None
            };
            let key = PrivateKeyWithHashAlg::new(Arc::new(key), hash);
            handle
                .authenticate_publickey(username, key)
                .await
                .map_err(|_| SshError::Protocol)?
        }
    };
    match result {
        AuthResult::Success => Ok(()),
        AuthResult::Failure { .. } => Err(SshError::AuthFailed),
    }
}

/// The `password` method, then, when the device refused it and offers `keyboard-interactive`
/// but not `password`, the password as the answer to each of its prompts (D-5).
async fn sign_in_with_password(
    handle: &mut Handle<Observer>,
    username: &str,
    password: &str,
) -> Result<AuthResult, SshError> {
    let result = handle
        .authenticate_password(username, password)
        .await
        .map_err(|_| SshError::Protocol)?;
    let AuthResult::Failure {
        remaining_methods, ..
    } = &result
    else {
        return Ok(result);
    };
    let prompts_only = remaining_methods.contains(&MethodKind::KeyboardInteractive)
        && !remaining_methods.contains(&MethodKind::Password);
    if !prompts_only {
        return Ok(result);
    }
    answer_prompts(handle, username, password).await
}

/// Answers every keyboard-interactive prompt with the password, for at most
/// [`PROMPT_ROUNDS`] rounds. The prompts are the device's text: never read, logged or kept.
async fn answer_prompts(
    handle: &mut Handle<Observer>,
    username: &str,
    password: &str,
) -> Result<AuthResult, SshError> {
    let mut reply = handle
        .authenticate_keyboard_interactive_start(username, None)
        .await
        .map_err(|_| SshError::Protocol)?;
    for _ in 0..PROMPT_ROUNDS {
        let prompts = match reply {
            KeyboardInteractiveAuthResponse::Success => return Ok(AuthResult::Success),
            KeyboardInteractiveAuthResponse::Failure { .. } => break,
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => prompts.len(),
        };
        let answers = vec![password.to_owned(); prompts];
        reply = handle
            .authenticate_keyboard_interactive_respond(answers)
            .await
            .map_err(|_| SshError::Protocol)?;
    }
    Err(SshError::AuthFailed)
}

/// Decodes an `OpenSSH`, PEM or `PuTTY` private key; the decoder's error is dropped unread.
fn private_key(key: &[u8], passphrase: Option<&[u8]>) -> Result<russh::keys::PrivateKey, SshError> {
    let key = std::str::from_utf8(key).map_err(|_| SshError::UnreadableKey)?;
    let passphrase = passphrase
        .map(std::str::from_utf8)
        .transpose()
        .map_err(|_| SshError::UnreadableKey)?;
    decode_secret_key(key, passphrase).map_err(|_| SshError::UnreadableKey)
}

/// The hub's side of the key exchange: reports the presented key once, and lets the
/// connection go on only when it is the expected key. A certificate, or a key the hub does
/// not accept, is reported as `None` and never matches (SEC-007, K4 Section 8.3).
pub(super) struct Observer {
    expected: Option<PublicKey>,
    seen: Option<oneshot::Sender<Option<PublicKey>>>,
}

impl Observer {
    /// An observer that lets the connection go on only for `expected`, and the receiver of
    /// the key the host presents. The receiver fails when no key exchange reached the key.
    pub(super) fn expecting(
        expected: Option<PublicKey>,
    ) -> (Self, oneshot::Receiver<Option<PublicKey>>) {
        let (seen, receiver) = oneshot::channel();
        let observer = Self {
            expected,
            seen: Some(seen),
        };
        (observer, receiver)
    }

    /// The key as the hub stores it, rebuilt from the bytes russh decoded: a certificate or a
    /// key the hub does not accept counts as no key.
    fn presented(presented: &PublicKeyOrCertificate) -> Option<PublicKey> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
            return None;
        };
        PublicKey::from_blob(key.to_bytes().ok()?).ok()
    }
}

impl client::Handler for Observer {
    type Error = russh::Error;

    /// russh has verified the exchange signature with this key before it calls this. Only
    /// the first exchange is reported; every exchange must present the expected key.
    fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let presented = Self::presented(server_public_key);
        let go_on = presented.is_some() && presented == self.expected;
        if let Some(seen) = self.seen.take() {
            // The receiver is gone only when the caller was dropped; nobody is left to tell.
            let _gone = seen.send(presented);
        }
        std::future::ready(Ok(go_on))
    }
}
