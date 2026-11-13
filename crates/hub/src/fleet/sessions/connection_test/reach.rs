//! Reaching an RDP or VNC port for a connection test: the hub's checked connect (R-032), the
//! one SSH uses, so the host is resolved once, every address passes the target rule and the
//! connection goes to a checked address. I/O edge; nothing here logs.

use std::{future::Future, time::Duration};

use tokio::net::TcpStream;

use crate::fleet::ssh::{self, SshError, Target};

/// Why a desktop protocol read gave nothing. Holds no host name or remote text (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(in crate::fleet::sessions) enum ReadError {
    #[error("the host name does not resolve")]
    Unresolved,
    #[error("the host resolves to an address sessions may not reach")]
    RefusedTarget,
    #[error("no connection to the host's port")]
    Unreachable,
    #[error("the host did not finish in time")]
    Timeout,
    #[error("the host did not answer as the protocol does")]
    Unexpected,
    #[error("the host did not complete a TLS handshake")]
    Tls,
}

/// A connection to the first checked address of `target` that answers.
///
/// # Errors
/// [`ReadError::Unresolved`], [`ReadError::RefusedTarget`] before any connection, or
/// [`ReadError::Unreachable`].
pub(in crate::fleet::sessions) async fn reach(target: Target<'_>) -> Result<TcpStream, ReadError> {
    ssh::connect(target).await.map_err(|error| match error {
        SshError::Unresolved => ReadError::Unresolved,
        SshError::RefusedTarget => ReadError::RefusedTarget,
        _ => ReadError::Unreachable,
    })
}

/// `read`, ended with [`ReadError::Timeout`] once `limit` passes.
///
/// # Errors
/// As `read`, or [`ReadError::Timeout`].
pub(in crate::fleet::sessions) async fn within<T>(
    limit: Duration,
    read: impl Future<Output = Result<T, ReadError>>,
) -> Result<T, ReadError> {
    tokio::time::timeout(limit, read)
        .await
        .map_err(|_| ReadError::Timeout)?
}
