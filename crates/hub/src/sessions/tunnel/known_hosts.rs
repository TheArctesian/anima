//! The `known_hosts` line guacd gets as its `host-key` argument (SEC-006, SEC-007, D-4).
//!
//! guacd reads the line with `libssh2_knownhost_readline` and checks the server's key with
//! `libssh2_knownhost_checkp(hostname, port, …)` (guacamole-server 1.5.5
//! `src/common-ssh/key.c:176` and `:216`; 1.6.0 `src/common-ssh/key.c:183` and `:223`).
//! libssh2 (1.11.0 `src/knownhost.c:370-384`, `:405-478`) first compares the plain host name
//! of each entry with `[<host>]:<port>`, then with `<host>`, using `strcmp`; then it
//! compares the entry's base64 key with the base64 of the server's key blob. So the line is
//! `<host> <algorithm> <base64>` for port 22 (as OpenSSH writes it) and
//! `[<host>]:<port> <algorithm> <base64>` otherwise, where `<host>` is byte for byte the
//! `hostname` argument.

use crate::sessions::PublicKey;

/// The SSH port OpenSSH writes without brackets.
const DEFAULT_PORT: u16 = 22;
/// libssh2 copies each host name into a 256-byte buffer and refuses one of 255 bytes or more
/// (1.11.0 `src/knownhost.c:638-645`).
const MAX_HOST_FIELD: usize = 254;

/// The device host cannot go into a `known_hosts` line. Holds no text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the device host cannot be written into a known_hosts line")]
pub(super) struct BadHost;

/// The line for `host` and `port` that pins `key`.
///
/// # Errors
/// [`BadHost`] when `host` holds a character that would change how libssh2
/// splits or reads the line (space, comma, `|`, `#`, `@`, brackets, control characters, or
/// anything outside `[A-Za-z0-9._:-]`), or is too long for libssh2.
pub(super) fn known_hosts_line(host: &str, port: u16, key: &PublicKey) -> Result<String, BadHost> {
    if host.is_empty() || !host.bytes().all(is_host_byte) {
        return Err(BadHost);
    }
    let field = if port == DEFAULT_PORT {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    };
    if field.len() > MAX_HOST_FIELD {
        return Err(BadHost);
    }
    Ok(format!("{field} {} {}", key.algorithm(), key.to_base64()))
}

/// A byte of a host name or an IP literal (devices allows nothing else, DEV-010).
fn is_host_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
}
