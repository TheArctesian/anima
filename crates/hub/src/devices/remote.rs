//! Remote access settings of a device: RDP, VNC, the transfer policy and where metrics come
//! from (RA-030..033, DEV-010). Every `Default` equals the column defaults of migration 0110
//! and the K1 input defaults (DB-011).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{num::NonZeroU8, str::FromStr};

use async_graphql::Enum;

use super::{mac::MacAddress, model::UnknownValue};

/// Generates `as_str` and `FromStr` for an enum stored as lower-case text.
macro_rules! stored_text {
    ($type:ident, $noun:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $type {
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }

        impl FromStr for $type {
            type Err = UnknownValue;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(UnknownValue::new($noun, text)),
                }
            }
        }
    };
}

/// How the RDP server and the hub agree on security, stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum RdpSecurity {
    Any,
    Nla,
    NlaExt,
    Tls,
    Vmconnect,
    Rdp,
}

stored_text!(RdpSecurity, "rdp security", {
    Any => "any", Nla => "nla", NlaExt => "nla_ext", Tls => "tls", Vmconnect => "vmconnect", Rdp => "rdp",
});

/// How the remote screen follows the window size, stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "RdpResizeMethod")]
pub enum ResizeMethod {
    DisplayUpdate,
    Reconnect,
}

stored_text!(ResizeMethod, "rdp resize method", {
    DisplayUpdate => "display_update", Reconnect => "reconnect",
});

/// Where a device's metrics come from, stored as lower-case text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
pub enum MetricsSource {
    #[default]
    None,
    Ssh,
}

stored_text!(MetricsSource, "metrics source", { None => "none", Ssh => "ssh" });

/// How to reach a device over RDP. The password lives in `credentials`, never here.
#[derive(Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "K3 fixes these fields; each is a separate switch in the form"
)]
pub struct RdpSettings {
    pub enabled: bool,
    pub port: u16,
    pub username: String,
    pub domain: Option<String>,
    pub security: RdpSecurity,
    pub ignore_certificate: bool,
    pub resize: ResizeMethod,
    pub audio_out: bool,
    pub microphone: bool,
    pub shared_drive: bool,
}

impl Default for RdpSettings {
    /// RDP off, port 3389, no user, NLA, certificate checked, audio out only.
    fn default() -> Self {
        Self {
            enabled: false,
            port: 3389,
            username: String::new(),
            domain: None,
            security: RdpSecurity::Nla,
            ignore_certificate: false,
            resize: ResizeMethod::DisplayUpdate,
            audio_out: true,
            microphone: false,
            shared_drive: false,
        }
    }
}

/// How to reach a device over VNC. The password lives in `credentials`, never here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VncSettings {
    pub enabled: bool,
    pub port: u16,
    pub username: Option<String>,
}

impl Default for VncSettings {
    /// VNC off, port 5900, no user.
    fn default() -> Self {
        Self {
            enabled: false,
            port: 5900,
            username: None,
        }
    }
}

/// What a remote session may move between the operator's computer and the device (RA-033).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "K3 fixes these fields; each is a separate switch in the form"
)]
pub struct TransferPolicy {
    pub upload: bool,
    pub download: bool,
    pub copy: bool,
    pub paste: bool,
}

impl Default for TransferPolicy {
    /// Everything allowed.
    fn default() -> Self {
        Self {
            upload: true,
            download: true,
            copy: true,
            paste: true,
        }
    }
}

/// RDP settings as typed in the form; `port` is the raw GraphQL integer, `None` when left out.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the form has one switch per field"
)]
pub(super) struct RdpInput {
    pub(super) enabled: bool,
    pub(super) port: Option<i32>,
    pub(super) username: String,
    pub(super) domain: Option<String>,
    pub(super) security: RdpSecurity,
    pub(super) ignore_certificate: bool,
    pub(super) resize: ResizeMethod,
    pub(super) audio_out: bool,
    pub(super) microphone: bool,
    pub(super) shared_drive: bool,
}

/// VNC settings as typed in the form.
#[derive(Clone, Debug)]
pub(super) struct VncInput {
    pub(super) enabled: bool,
    pub(super) port: Option<i32>,
    pub(super) username: Option<String>,
}

/// The remote access part of a device create or update, as typed in the form. On an update an
/// absent part is left as it is and `mac_address` of `""` clears the address (D-2).
#[derive(Clone, Debug, Default)]
pub(super) struct RemoteForm {
    pub(super) rdp: Option<RdpInput>,
    pub(super) vnc: Option<VncInput>,
    pub(super) mac_address: Option<String>,
    pub(super) metrics_source: Option<MetricsSource>,
    pub(super) transfer: Option<TransferPolicy>,
    /// On a create `null` is no limit; on an update `null` keeps the limit and `0` clears it.
    pub(super) session_limit: Option<i32>,
    pub(super) record_sessions: Option<bool>,
}

/// The validated remote access part of a device to create.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RemoteAccess {
    pub(super) rdp: RdpSettings,
    pub(super) vnc: VncSettings,
    pub(super) mac_address: Option<MacAddress>,
    pub(super) metrics_source: MetricsSource,
    pub(super) transfer: TransferPolicy,
    /// `None` is no limit on simultaneous sessions (RA-036).
    pub(super) session_limit: Option<NonZeroU8>,
    pub(super) record_sessions: bool,
}

/// The validated remote access part of a device update: `None` leaves the field as it is, and
/// `Some(None)` clears the address or the session limit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RemoteChange {
    pub(super) rdp: Option<RdpSettings>,
    pub(super) vnc: Option<VncSettings>,
    #[expect(
        clippy::option_option,
        reason = "absent leaves the address, null clears it, a value sets it"
    )]
    pub(super) mac_address: Option<Option<MacAddress>>,
    pub(super) metrics_source: Option<MetricsSource>,
    pub(super) transfer: Option<TransferPolicy>,
    #[expect(
        clippy::option_option,
        reason = "absent keeps the limit, null clears it, a value sets it"
    )]
    pub(super) session_limit: Option<Option<NonZeroU8>>,
    pub(super) record_sessions: Option<bool>,
}
