//! What the hub tells guacd about one SSH session: the `connect` values and the time zone
//! (K4 Section 3, GW-012, GW-013, RA-010, RA-011).
//!
//! Every value comes from the device record, the credential store, the accepted host key or
//! a constant here. The client supplies only the display and the time zone, and the time zone
//! is used only when it is zone-shaped.

use std::net::IpAddr;

use guac_protocol::{ArgValues, is_zone_name};

use super::known_hosts::known_hosts_line;
use crate::credentials::{Secret, SshSecret};
use crate::devices::Device;
use crate::sessions::HostKey;

/// The terminal font the gateway image ships (RA-011, spec Section 6.3.1).
const FONT_NAME: &str = "OpenDyslexicM Nerd Font Mono";
const FONT_SIZE: &str = "12";
const SCROLLBACK_LINES: &str = "5000";
const TERMINAL_TYPE: &str = "xterm-256color";
/// Seconds between SSH keep-alive messages (spec Section 6.3.1).
const SERVER_ALIVE_SECONDS: &str = "30";
/// The time zone when the client's is not zone-shaped (sites have no zone in Phase 1, R-032).
const FALLBACK_ZONE: &str = "UTC";

/// Why the hub cannot build the `connect` values. Holds no value (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BadConnectValue {
    #[error("the stored {argument} is not UTF-8 text")]
    NotText { argument: &'static str },
}

/// The `connect` values by guacd argument name for an SSH session to `device` at `target`,
/// pinned to `host_key` (SEC-007). `target` is the address the hub resolved the device's host
/// to and checked (R-032): guacd gets it as `hostname` and the `host-key` line names it, so
/// guacd never resolves the name again or reaches an address the hub refused. Names not set
/// here get an empty value from the handshake, so
/// `enable-sftp`, `recording-path`, `typescript-path`, guacd's own `timezone` argument (the
/// `timezone` instruction sets it, WP-08 H-9) and every path stay empty.
///
/// A host key is required: with an empty `host-key` guacd checks no key and would send the
/// credential to any host (SEC-006). Without a stored secret guacd prompts in the terminal.
///
/// # Errors
/// [`BadConnectValue::NotText`] when a stored secret is not UTF-8.
pub(super) fn connect_values(
    device: &Device,
    target: IpAddr,
    secret: Option<&SshSecret>,
    host_key: &HostKey,
) -> Result<ArgValues, BadConnectValue> {
    let host_key = known_hosts_line(target, device.ssh.port, &host_key.public_key);
    let mut values = ArgValues::new();
    values.set("hostname", target.to_string());
    values.set("port", device.ssh.port.to_string());
    values.set("username", device.ssh.username.as_str());
    values.set("host-key", host_key);
    values.set("color-scheme", anima_theme::guac_color_scheme());
    values.set("font-name", FONT_NAME);
    values.set("font-size", FONT_SIZE);
    values.set("scrollback", SCROLLBACK_LINES);
    values.set("terminal-type", TERMINAL_TYPE);
    values.set("server-alive-interval", SERVER_ALIVE_SECONDS);
    match secret {
        None => {}
        Some(SshSecret::Password(password)) => {
            values.set("password", text("password", password)?);
        }
        Some(SshSecret::PrivateKey { key, passphrase }) => {
            values.set("private-key", text("private-key", key)?);
            if let Some(passphrase) = passphrase {
                values.set("passphrase", text("passphrase", passphrase)?);
            }
        }
    }
    Ok(values)
}

/// The time zone for the handshake: the client's when it is zone-shaped, else UTC (K4
/// Section 3, R-032 E-6). The handshake refuses any other text, so this never fails.
pub(super) fn time_zone(requested: Option<&str>) -> String {
    requested
        .filter(|zone| is_zone_name(zone))
        .unwrap_or(FALLBACK_ZONE)
        .to_owned()
}

/// The secret as text, borrowed. `ArgValues::set` then copies it into a plain `String`
/// that is not zeroed on drop (guac-protocol, issue #1).
fn text<'a>(argument: &'static str, secret: &'a Secret) -> Result<&'a str, BadConnectValue> {
    std::str::from_utf8(secret.expose()).map_err(|_| BadConnectValue::NotText { argument })
}
