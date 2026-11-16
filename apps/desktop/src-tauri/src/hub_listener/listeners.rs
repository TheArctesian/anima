//! Who listens on a loopback port, read from what the system lists (F-10). Pure: text in,
//! listeners out, and the verdict over them.
//!
//! Linux lists every user's sockets in `/proc/net/tcp` and `/proc/net/tcp6` with their owner.
//! macOS `lsof`, run as this user, lists only this user's processes.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::Serialize;

/// The state number of a listening socket in `/proc/net/tcp`.
const LISTEN: &str = "0A";

/// Who listens on the port of a hub address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Owner {
    /// A process of this user; the client may send its token and password.
    Own,
    /// A process of another user, or one that is not the app's own device link.
    Other,
    /// Nothing.
    #[serde(rename = "none")]
    Nobody,
}

/// The address a socket listens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Every address of every family (`lsof`'s `*`).
    Any,
    At(IpAddr),
}

impl Bound {
    /// Whether a connection to `target` can reach this socket. An unspecified address
    /// (`0.0.0.0`, `::`) covers its own family only.
    pub fn covers(self, target: IpAddr) -> bool {
        match self {
            Self::Any => true,
            Self::At(address) if address.is_unspecified() => address.is_ipv4() == target.is_ipv4(),
            Self::At(address) => address == target,
        }
    }
}

/// One listening socket: its address, port and, where the system tells it, its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listener {
    pub bound: Bound,
    pub port: u16,
    pub uid: Option<u32>,
}

/// The listening sockets in the text of `/proc/net/tcp` (`ipv6` false) or `/proc/net/tcp6`.
/// A line that is not of the form is passed over.
pub fn proc_net_tcp(text: &str, ipv6: bool) -> Vec<Listener> {
    text.lines()
        .skip(1)
        .filter_map(|line| proc_line(line, ipv6))
        .collect()
}

fn proc_line(line: &str, ipv6: bool) -> Option<Listener> {
    let fields: Vec<&str> = line.split_ascii_whitespace().collect();
    // sl local_address rem_address st tx:rx tr:when retrnsmt uid ...
    let (local, state, uid) = (fields.get(1)?, fields.get(3)?, fields.get(7)?);
    if *state != LISTEN {
        return None;
    }
    let (address, port) = local.split_once(':')?;
    let address = if ipv6 {
        IpAddr::V6(proc_ipv6(address)?)
    } else {
        IpAddr::V4(Ipv4Addr::from(proc_word(address)?))
    };
    Some(Listener {
        bound: Bound::At(address),
        port: u16::from_str_radix(port, 16).ok()?,
        uid: Some(uid.parse().ok()?),
    })
}

/// One 32-bit word as the kernel prints it: eight hexadecimal digits of the word in this
/// machine's byte order.
fn proc_word(hex: &str) -> Option<[u8; 4]> {
    if hex.len() != 8 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(u32::from_str_radix(hex, 16).ok()?.to_ne_bytes())
}

fn proc_ipv6(hex: &str) -> Option<Ipv6Addr> {
    if hex.len() != 32 || !hex.is_ascii() {
        return None;
    }
    let mut octets = [0_u8; 16];
    for (word, chunk) in octets.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        *chunk = proc_word(hex.get(word * 8..word * 8 + 8)?)?;
    }
    Some(Ipv6Addr::from(octets))
}

/// The listening sockets in `lsof -F n` output: lines `n<address>:<port>`, the address `*`,
/// an IPv4 address or an IPv6 address in brackets. Other lines are passed over. `lsof` gives
/// no owner; the caller asked it for this user's processes only.
pub fn lsof_names(text: &str) -> Vec<Listener> {
    text.lines()
        .filter_map(|line| line.strip_prefix('n'))
        .filter_map(|name| {
            let (address, port) = name.rsplit_once(':')?;
            let bound = if address == "*" {
                Bound::Any
            } else {
                let address = address
                    .strip_prefix('[')
                    .and_then(|inner| inner.strip_suffix(']'))
                    .unwrap_or(address);
                Bound::At(address.parse().ok()?)
            };
            Some(Listener {
                bound,
                port: port.parse().ok()?,
                uid: None,
            })
        })
        .collect()
}

/// Whether `listeners` let a connection to `target`:`port` reach only this user: at least one
/// covers it, and each one that covers it is this user's (`uid`, or no owner told).
pub fn only_own(listeners: &[Listener], target: IpAddr, port: u16, uid: u32) -> bool {
    let mut covering = listeners
        .iter()
        .filter(|listener| listener.port == port && listener.bound.covers(target))
        .peekable();
    covering.peek().is_some()
        && covering.all(|listener| listener.uid.is_none_or(|owner| owner == uid))
}

/// What one address of the hub origin gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// A connection to it was accepted.
    pub accepts: bool,
    /// Only this user's processes listen on it.
    pub own: bool,
}

/// The verdict over every address the origin can reach: `Own` only when each address that
/// someone listens on is this user's alone.
pub fn verdict(targets: &[Target]) -> Owner {
    let mut listened = targets
        .iter()
        .filter(|target| target.accepts || target.own)
        .peekable();
    if listened.peek().is_none() {
        return Owner::Nobody;
    }
    if listened.all(|target| target.own) {
        Owner::Own
    } else {
        Owner::Other
    }
}
