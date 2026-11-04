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
//!
//! The host is always the address the hub checked (R-032), never a name. An IP address in its
//! standard text form holds only hex digits, dots and colons and is at most 39 bytes, so it
//! can neither change how libssh2 splits the line nor overflow its 256-byte host buffer
//! (1.11.0 `src/knownhost.c:638-645`).

use std::net::IpAddr;

use crate::sessions::PublicKey;

/// The SSH port OpenSSH writes without brackets.
const DEFAULT_PORT: u16 = 22;

/// The line for `host` and `port` that pins `key`.
pub(super) fn known_hosts_line(host: IpAddr, port: u16, key: &PublicKey) -> String {
    let field = if port == DEFAULT_PORT {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };
    format!("{field} {} {}", key.algorithm(), key.to_base64())
}
