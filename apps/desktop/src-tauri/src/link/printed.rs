//! What `anima-link` takes and prints, read without trust (2.6). Pure: text in, values out.
//! An invite is a secret; nothing here keeps or quotes one.

use std::net::SocketAddr;

use zeroize::Zeroizing;

use super::LinkError;

const INVITE_PREFIX: &str = "anima-invite.1.";
const MOST_INVITE: usize = 4096;
const JOINED: &str = "This device has joined the hub. Its key fingerprint is ";
const READY: &str = "msg=\"the device link is ready\"";
const NO_ANSWER: &str = "the hub did not answer";
const REFUSED: &str = "the hub refused this device";
const PORT_TAKEN: &str = "address already in use";

/// The invite as `join` reads it: one line. Space around a pasted invite is dropped; anything
/// but the invite alphabet inside it is refused.
pub fn invite(pasted: &str) -> Result<Zeroizing<String>, LinkError> {
    let invite = pasted.trim_matches(|c: char| c.is_ascii_whitespace());
    let fits = invite.len() <= MOST_INVITE
        && invite.len() > INVITE_PREFIX.len()
        && invite.starts_with(INVITE_PREFIX)
        && invite
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'));
    if !fits {
        return Err(LinkError::BadInvite);
    }
    Ok(Zeroizing::new(format!("{invite}\n")))
}

/// The key fingerprint in `join`'s answer: hexadecimal, 8 to 64 digits.
pub fn fingerprint(printed: &str) -> Option<String> {
    let rest = printed.lines().find_map(|line| line.strip_prefix(JOINED))?;
    let fingerprint = rest.strip_suffix('.')?;
    let fits = (8..=64).contains(&fingerprint.len())
        && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit());
    fits.then(|| fingerprint.to_owned())
}

/// Why `join` failed, from what it said on standard error.
pub fn join_failure(said: &str) -> LinkError {
    if said.contains(NO_ANSWER) {
        LinkError::NoAnswer
    } else if said.contains(REFUSED) {
        LinkError::Refused
    } else {
        LinkError::Failed
    }
}

/// What one line of the running device link's log tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceLine {
    /// Ready, listening on this loopback port.
    Ready(u16),
    /// The local port is taken by another program.
    PortTaken,
}

/// Reads one log line of `anima-link device`; most lines tell nothing to the shell.
pub fn device_line(line: &str) -> Option<DeviceLine> {
    if line.contains(PORT_TAKEN) {
        return Some(DeviceLine::PortTaken);
    }
    if !line.contains(READY) {
        return None;
    }
    let local = line
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("local="))?;
    let address: SocketAddr = local.trim_matches('"').parse().ok()?;
    (address.ip().is_loopback() && address.port() != 0).then_some(DeviceLine::Ready(address.port()))
}

/// The path and latency in `ctl status`'s answer: `{ role: "device", path, latencyMs }`.
pub fn link_path(printed: &[u8]) -> Option<(String, f64)> {
    let status: serde_json::Value = serde_json::from_slice(printed).ok()?;
    if status.get("role")?.as_str()? != "device" {
        return None;
    }
    let path = status.get("path")?.as_str()?;
    let latency = status.get("latencyMs")?.as_f64()?;
    let known = matches!(path, "direct" | "relayed" | "none");
    (known && latency.is_finite() && latency >= 0.0).then(|| (path.to_owned(), latency))
}
