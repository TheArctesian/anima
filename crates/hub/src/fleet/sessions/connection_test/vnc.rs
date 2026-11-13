//! Reading a VNC server's version banner (RFC 6143 7.1.1) for a connection test: the 12 bytes
//! `RFB xxx.yyy\n` it sends first. The hub sends nothing, so the server never reaches its
//! security handshake. I/O edge; nothing here logs, and no error holds the banner.

use std::time::Duration;

use tokio::io::AsyncReadExt as _;

use super::reach::{ReadError, reach, within};
use crate::fleet::ssh::Target;

/// The banner's length: `RFB `, three digits, `.`, three digits, a newline.
pub(in crate::fleet::sessions) const BANNER: usize = 12;

/// Whether `bytes` is an RFB version banner (`RFB 003.008\n`, or another version number).
pub(in crate::fleet::sessions) fn is_banner(bytes: &[u8; BANNER]) -> bool {
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    bytes.starts_with(b"RFB ")
        && digits(4..7)
        && bytes[7] == b'.'
        && digits(8..11)
        && bytes[11] == b'\n'
}

/// Reads `target`'s banner within `limit`, then closes.
///
/// # Errors
/// As [`super::rdp::read_server_certificate`] for the connection; [`ReadError::Unexpected`]
/// when the first 12 bytes are not a banner.
pub(in crate::fleet::sessions) async fn read_banner(
    target: Target<'_>,
    limit: Duration,
) -> Result<(), ReadError> {
    within(limit, async {
        let mut stream = reach(target).await?;
        let mut banner = [0; BANNER];
        stream
            .read_exact(&mut banner)
            .await
            .map_err(|_| ReadError::Unexpected)?;
        if is_banner(&banner) {
            Ok(())
        } else {
            Err(ReadError::Unexpected)
        }
    })
    .await
}
