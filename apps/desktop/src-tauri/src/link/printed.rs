//! What `anima-link` takes and prints, read without trust (2.6). Pure: text in, values out.
//! An invite is a secret; nothing here keeps or quotes one.

use std::{net::SocketAddr, path::PathBuf};

use zeroize::Zeroizing;

use super::LinkError;

const INVITE_PREFIX: &str = "anima-invite.1.";
const MOST_INVITE: usize = 4096;
/// The first words of `join`'s answer before the fingerprint (`link/cmd/anima-link/join.go`).
pub const JOINED: &str = "This device has joined the hub. Its key fingerprint is ";
/// The `msg` of the device link's ready line, which `link/cmd/anima-link/device.go` logs with
/// `local=<address>` and `key=<fingerprint>` through `slog`'s text handler. The parser, its
/// tests and the stand-in programs of the tests take it from here.
pub const READY: &str = "msg=\"the device link is ready\"";
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

/// The loopback port in `ctl status`'s answer of a running device link: `{ role: "device",
/// local: "127.0.0.1:<port>" }`.
pub fn link_local(printed: &[u8]) -> Option<u16> {
    let status: serde_json::Value = serde_json::from_slice(printed).ok()?;
    if status.get("role")?.as_str()? != "device" {
        return None;
    }
    let address: SocketAddr = status.get("local")?.as_str()?.parse().ok()?;
    (address.ip().is_loopback() && address.port() != 0).then_some(address.port())
}

/// Whether `unit show` says that the unit of the user is installed: `{ installed: true }`.
pub fn unit_installed(printed: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(printed)
        .ok()
        .and_then(|shown| shown.get("installed")?.as_bool())
        .unwrap_or(false)
}

/// The file of the unit in the answer of `unit show`, when the unit is installed.
pub fn unit_file(printed: &[u8]) -> Option<PathBuf> {
    let shown: serde_json::Value = serde_json::from_slice(printed).ok()?;
    let file = PathBuf::from(shown.get("file")?.as_str()?);
    (unit_installed(printed) && file.is_absolute()).then_some(file)
}

/// Whether the text of a unit file runs the link of `folder`: the folder is one whole argument
/// of it, a `<string>` of a launch agent or a quoted word of a systemd unit. A unit of a
/// different folder (a second copy of the app, a test) is not the unit of this one.
pub fn unit_runs_folder(unit: &str, folder: &str) -> bool {
    unit.contains(&format!("<string>{folder}</string>")) || unit.contains(&format!("\"{folder}\""))
}

/// The path and latency in `ctl status`'s answer: `{ role: "device", path, latencyMs }`.
pub fn link_path(printed: &[u8]) -> Option<(String, f64, Option<String>)> {
    let status: serde_json::Value = serde_json::from_slice(printed).ok()?;
    if status.get("role")?.as_str()? != "device" {
        return None;
    }
    let path = status.get("path")?.as_str()?;
    let latency = status.get("latencyMs")?.as_f64()?;
    let known = matches!(path, "direct" | "relayed" | "none");
    let key = status
        .get("publicKey")
        .and_then(serde_json::Value::as_str)
        .filter(|key| key.starts_with("nodekey:") && key.len() == "nodekey:".len() + 64)
        .map(str::to_owned);
    (known && latency.is_finite() && latency >= 0.0).then(|| (path.to_owned(), latency, key))
}
