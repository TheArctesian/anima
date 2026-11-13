//! RDP and VNC passwords and their pure mapping to stored rows: one sealed row per kind
//! (SEC-003, DEV-011).

use crate::ids::DeviceId;

/// Which remote-desktop password a row holds; the `kind` column of `credentials.secrets`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordKind {
    Rdp,
    Vnc,
}

impl PasswordKind {
    /// The `kind` column value.
    pub(super) const fn as_sql(self) -> &'static str {
        match self {
            Self::Rdp => "rdp_password",
            Self::Vnc => "vnc_password",
        }
    }
}

/// The associated data that binds a sealed password to its device and kind, so a value moved
/// to another device or kind does not open:
/// `credentials.secrets:<device_id>:<kind>`, the device id as hyphenated lowercase UUID text.
/// Rows sealed since SEC-011 use the host-bound context of `host` instead; this one opens only
/// rows sealed before it.
pub(super) fn context(device: DeviceId, kind: PasswordKind) -> Vec<u8> {
    format!("credentials.secrets:{device}:{}", kind.as_sql()).into_bytes()
}
