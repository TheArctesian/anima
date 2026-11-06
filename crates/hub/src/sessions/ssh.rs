//! The hub's SSH client side, shared by the connection test and the terminal (R-034, SEC-007):
//! the host key order and the checked connect (R-032) here, the handler that pins the
//! presented key (`observer`) and the sign-in with a stored credential (`sign_in`, OD-9).
//! I/O edge.
//!
//! The host is resolved first and every address it resolves to must pass the target rule
//! (R-032); the connection then goes to a checked address, never to the name again, so a
//! second lookup cannot point it somewhere else. Nothing here logs, and no error holds a
//! remote text, a host name or a secret (LOG-007).

mod observer;
mod sign_in;

use std::{borrow::Cow, net::SocketAddr};

use russh::{
    Preferred,
    keys::{Algorithm, EcdsaCurve, HashAlg},
};
use tokio::net::TcpStream;

use super::target::TargetRule;

pub(super) use observer::Observer;
pub(super) use sign_in::authenticate;

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
