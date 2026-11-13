//! What the hub tells guacd about one RDP or VNC session: the protocol it selects, the
//! `connect` values, the audio the client plays and the time zone (K4 Section 3, GW-012,
//! GW-013, RA-021, RA-033).
//!
//! Every value comes from the device record, the credential store, the accepted server
//! certificate, the hub's configuration or a constant here. The client supplies only the
//! display and the time zone, and the time zone is used only when it is zone-shaped. Names
//! not set here get an empty value from the handshake, so every `wol-*`, `sftp-*` and
//! `recording-*` name, `cert-tofu`, guacd's own `timezone` argument (the `timezone`
//! instruction sets it) and every path but the configured drive stay empty.
//!
//! guacd reads a boolean argument as true only when it is exactly `true`; empty is its
//! default, false for every name set here (guacamole-server `guac_user_parse_args_boolean`).

use std::net::IpAddr;

use guac_protocol::{ArgValues, is_zone_name};

use crate::fleet::credentials::{PasswordKind, Secret};
use crate::fleet::devices::{Device, RdpSecurity, ResizeMethod, TransferPolicy};
use crate::fleet::{sessions::Desktop, trust::ServerCertificate};

/// The time zone when the client's is not zone-shaped (sites have no zone, R-032).
const FALLBACK_ZONE: &str = "UTC";
/// guacd's text for a true boolean argument.
const TRUE: &str = "true";
/// The audio the client plays when the device sends sound (K4 Section 3).
const SOUND: &[&str] = &["audio/L8", "audio/L16"];
/// The hash `FreeRDP` names in front of an accepted fingerprint.
const FINGERPRINT_HASH: &str = "sha256";

impl Desktop {
    /// The name the handshake's `select` sends (K4 Section 3 step 1).
    pub(super) const fn select(self) -> &'static str {
        match self {
            Self::Rdp => "rdp",
            Self::Vnc => "vnc",
        }
    }

    /// Which stored password signs in.
    pub(in crate::fleet::sessions) const fn password(self) -> PasswordKind {
        match self {
            Self::Rdp => PasswordKind::Rdp,
            Self::Vnc => PasswordKind::Vnc,
        }
    }

    /// The port in the device's settings for this protocol.
    pub(super) const fn port(self, device: &Device) -> u16 {
        match self {
            Self::Rdp => device.rdp.port,
            Self::Vnc => device.vnc.port,
        }
    }
}

/// Why the hub cannot build the `connect` values. Holds no value (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BadConnectValue {
    #[error("the stored {argument} is not UTF-8 text")]
    NotText { argument: &'static str },
}

/// One session's sources for its `connect` values.
pub(super) struct Sources<'a> {
    pub(super) device: &'a Device,
    /// The address the hub resolved the device's host to and checked (R-032): guacd gets it
    /// as `hostname`, so it never resolves the name again or reaches an address the hub
    /// refused.
    pub(super) target: IpAddr,
    /// The stored password; without one guacd asks the client (GW-015).
    pub(super) password: Option<&'a Secret>,
    /// The accepted RDP server certificate, if any (SEC-007).
    pub(super) certificate: Option<&'a ServerCertificate>,
    /// `ANIMA_GUACD_DRIVE_ROOT`, a path inside the guacd container (RA-021).
    pub(super) drive_root: &'a str,
}

/// The `connect` values by guacd argument name for `desktop` (K4 Section 3).
///
/// # Errors
/// [`BadConnectValue::NotText`] when the stored password is not UTF-8.
pub(super) fn connect_values(
    desktop: Desktop,
    sources: &Sources<'_>,
) -> Result<ArgValues, BadConnectValue> {
    let device = sources.device;
    let mut values = ArgValues::new();
    values.set("hostname", sources.target.to_string());
    values.set("port", desktop.port(device).to_string());
    if let Some(password) = sources.password {
        values.set("password", text("password", password)?);
    }
    clipboard(&mut values, device.transfer);
    match desktop {
        Desktop::Rdp => rdp(&mut values, sources),
        Desktop::Vnc => {
            if let Some(username) = &device.vnc.username {
                values.set("username", username.as_str());
            }
        }
    }
    Ok(values)
}

/// The RDP-only values: sign-in, security, display, sound and the shared drive.
fn rdp(values: &mut ArgValues, sources: &Sources<'_>) {
    let device = sources.device;
    let rdp = &device.rdp;
    values.set("username", rdp.username.as_str());
    if let Some(domain) = &rdp.domain {
        values.set("domain", domain.as_str());
    }
    values.set("security", security(rdp.security));
    flag(values, "ignore-cert", rdp.ignore_certificate);
    if let Some(certificate) = sources.certificate {
        values.set("cert-fingerprints", accepted_fingerprint(certificate));
    }
    values.set("resize-method", resize_method(rdp.resize));
    flag(values, "disable-audio", !rdp.audio_out);
    flag(values, "enable-audio-input", rdp.microphone);
    flag(values, "disable-download", !device.transfer.download);
    flag(values, "disable-upload", !device.transfer.upload);
    if rdp.shared_drive {
        flag(values, "enable-drive", true);
        values.set("drive-path", drive_path(sources.drive_root, device));
        flag(values, "create-drive-path", true);
    }
}

/// guacd's clipboard switches from the device's policy (RA-033); the relay is the first
/// guard, these the second.
fn clipboard(values: &mut ArgValues, transfer: TransferPolicy) {
    flag(values, "disable-copy", !transfer.copy);
    flag(values, "disable-paste", !transfer.paste);
}

/// The device's own folder under the drive root: `<root>/<device id>` (RA-021). The id is a
/// hyphenated UUID, so it cannot climb out of the root.
pub(super) fn drive_path(root: &str, device: &Device) -> String {
    format!("{}/{}", root.trim_end_matches('/'), device.id)
}

/// The accepted fingerprint as `FreeRDP` reads `CertificateAcceptedFingerprints`:
/// `<hash>:<hex pairs joined by colons>`, so `sha256:` before the stored form. `FreeRDP` splits
/// the hash name at the first colon and compares the rest case-insensitively with its own
/// colon-joined hex of the certificate.
pub(super) fn accepted_fingerprint(certificate: &ServerCertificate) -> String {
    format!("{FINGERPRINT_HASH}:{}", certificate.fingerprint_sha256)
}

/// The security guacd gets once the hub has read the accepted certificate over `chosen`, the
/// TLS security the server picked (SEC-007): `ANY` becomes `chosen`, because with `any` guacd
/// may settle on standard RDP security, which checks no certificate. Every other setting
/// already rules standard security out and is kept.
pub(super) const fn pinned_security(set: RdpSecurity, chosen: RdpSecurity) -> RdpSecurity {
    match set {
        RdpSecurity::Any => chosen,
        other => other,
    }
}

/// guacd's `security` text (guacamole-server `src/protocols/rdp/settings.c`).
const fn security(security: RdpSecurity) -> &'static str {
    match security {
        RdpSecurity::Any => "any",
        RdpSecurity::Nla => "nla",
        RdpSecurity::NlaExt => "nla-ext",
        RdpSecurity::Tls => "tls",
        RdpSecurity::Vmconnect => "vmconnect",
        RdpSecurity::Rdp => "rdp",
    }
}

/// guacd's `resize-method` text.
const fn resize_method(resize: ResizeMethod) -> &'static str {
    match resize {
        ResizeMethod::DisplayUpdate => "display-update",
        ResizeMethod::Reconnect => "reconnect",
    }
}

/// Sets `name` to `true` when `on`; left unset, the handshake sends it empty.
fn flag(values: &mut ArgValues, name: &str, on: bool) {
    if on {
        values.set(name, TRUE);
    }
}

/// What the `audio` instruction lists: the client plays sound only from an RDP device that
/// sends it (K4 Section 3).
pub(super) const fn audio(desktop: Desktop, device: &Device) -> &'static [&'static str] {
    match desktop {
        Desktop::Rdp if device.rdp.audio_out => SOUND,
        Desktop::Rdp | Desktop::Vnc => &[],
    }
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
