//! Who listens on a loopback port, read from what the system lists (F-10). Pure: text in,
//! listeners out, and the verdict over them.
//!
//! Linux lists every user's sockets in `/proc/net/tcp` and `/proc/net/tcp6` with their owner.
//! macOS `lsof`, run as this user, lists only this user's processes.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::Serialize;

use crate::link::DeviceState;

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

/// Which listeners the system's list holds, which decides how far a wildcard reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listed {
    /// Every user's (Linux `/proc/net`). A wildcard `::` reaches IPv4 too (dual stack), so a
    /// stranger's `::` listener counts against an IPv4 address.
    EveryUser,
    /// This user's only (macOS `lsof`). A wildcard reaches its own family only, so this user's
    /// `::` listener never vouches for an IPv4 address.
    ThisUser,
}

/// Whether a socket bound to `bound` can take a connection to `target`.
pub fn covers(bound: IpAddr, target: IpAddr, listed: Listed) -> bool {
    let (bound, target) = (bound.to_canonical(), target.to_canonical());
    match bound {
        IpAddr::V4(address) if address.is_unspecified() => target.is_ipv4(),
        IpAddr::V6(address) if address.is_unspecified() => {
            target.is_ipv6() || listed == Listed::EveryUser
        }
        address => address == target,
    }
}

/// One listening socket: its address, port and, where the system tells it, its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listener {
    pub address: IpAddr,
    pub port: u16,
    pub uid: Option<u32>,
}

/// The listening sockets in the text of `/proc/net/tcp` (`ipv6` false) or `/proc/net/tcp6`.
/// A line that is not of the form is passed over. An IPv4-mapped address (`::ffff:a.b.c.d`)
/// is read as the IPv4 address.
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
        IpAddr::V6(proc_ipv6(address)?).to_canonical()
    } else {
        IpAddr::V4(Ipv4Addr::from(proc_word(address)?))
    };
    Some(Listener {
        address,
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

/// The listening sockets in `lsof -F nt` output: a `t` line (`tIPv4` or `tIPv6`) before each
/// `n` line (`n<address>:<port>`, the address `*`, an IPv4 address or an IPv6 address in
/// brackets). `*` is the unspecified address of the `t` family; a `*` of no known family is
/// passed over. `lsof` gives no owner; the caller asked it for this user's processes only.
pub fn lsof_names(text: &str) -> Vec<Listener> {
    let mut family: Option<IpAddr> = None;
    let mut found = Vec::new();
    for line in text.lines() {
        if let Some(kind) = line.strip_prefix('t') {
            family = match kind {
                "IPv4" => Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                "IPv6" => Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
                _ => None,
            };
        } else if let Some(name) = line.strip_prefix('n') {
            found.extend(lsof_name(name, family));
        } else if line.starts_with('f') || line.starts_with('p') {
            family = None;
        }
    }
    found
}

fn lsof_name(name: &str, wildcard: Option<IpAddr>) -> Option<Listener> {
    let (address, port) = name.rsplit_once(':')?;
    let address = if address == "*" {
        wildcard?
    } else {
        address
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .unwrap_or(address)
            .parse()
            .ok()?
    };
    Some(Listener {
        address,
        port: port.parse().ok()?,
        uid: None,
    })
}

/// Whether `listeners` let a connection to `target`:`port` reach only this user: at least one
/// covers it, and each one that covers it is this user's (`uid`, or no owner told).
pub fn only_own(
    listeners: &[Listener],
    listed: Listed,
    (target, port): (IpAddr, u16),
    uid: u32,
) -> bool {
    let mut covering = listeners
        .iter()
        .filter(|listener| listener.port == port && covers(listener.address, target, listed))
        .peekable();
    covering.peek().is_some()
        && covering.all(|listener| listener.uid.is_none_or(|owner| owner == uid))
}

/// Whether the app's own device link permits `port` to be `own`: a port that is not the device
/// link's saved port, or the device link up on the port it reported (F-10).
pub fn link_permits(link: &DeviceState, port: u16) -> bool {
    link.port != Some(port) || link.serves(port)
}

/// The verdict: `accepts` when a connection was accepted, `own` when only this user's
/// processes listen. A listener of this user that did not accept in time still counts as one.
pub fn verdict(accepts: bool, own: bool) -> Owner {
    match (accepts, own) {
        (_, true) => Owner::Own,
        (true, false) => Owner::Other,
        (false, false) => Owner::Nobody,
    }
}
