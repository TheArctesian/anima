//! Domain types of the device inventory (DEV-010).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{net::IpAddr, num::NonZeroU8, str::FromStr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use ipnet::IpNet;

use super::{
    mac::MacAddress,
    remote::{
        MetricsSource, RdpSettings, RemoteAccess, RemoteChange, RemoteForm, TransferPolicy,
        VncSettings,
    },
    text::ip_literal,
};
use crate::{
    ids::{DeviceId, SiteId},
    scalars::Cidr,
};

/// The database holds text that is not a known value of an enum.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} '{text}'")]
pub struct UnknownValue {
    kind: &'static str,
    text: String,
}

impl UnknownValue {
    pub(super) fn new(kind: &'static str, text: &str) -> Self {
        Self {
            kind,
            text: text.to_owned(),
        }
    }
}

/// Operating system of a device, stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum OperatingSystem {
    Linux,
    Macos,
    Windows,
    Other,
}

impl OperatingSystem {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Other => "other",
        }
    }
}

impl FromStr for OperatingSystem {
    type Err = UnknownValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::Macos),
            "windows" => Ok(Self::Windows),
            "other" => Ok(Self::Other),
            _ => Err(UnknownValue {
                kind: "operating system",
                text: text.to_owned(),
            }),
        }
    }
}

/// The Linux distribution a device runs: the `ID` of its os-release, 1 to 32 characters of
/// `a-z`, `0-9`, `.`, `_` and `-` (DEV-016). Text from the device, so nothing else is kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Distribution(String);

impl Distribution {
    /// `text` when it is a valid distribution id; `None` for anything else.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let allowed = |byte: u8| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        };
        let valid = (1..=32).contains(&text.len()) && text.bytes().all(allowed);
        valid.then(|| Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How the hub signs in over SSH, stored as lower-case text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum SshAuth {
    Password,
    PrivateKey,
}

impl SshAuth {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::PrivateKey => "private_key",
        }
    }
}

impl FromStr for SshAuth {
    type Err = UnknownValue;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "password" => Ok(Self::Password),
            "private_key" => Ok(Self::PrivateKey),
            _ => Err(UnknownValue {
                kind: "ssh auth",
                text: text.to_owned(),
            }),
        }
    }
}

/// How to reach a device over SSH. The secret lives in `credentials`, never here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshSettings {
    pub enabled: bool,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
}

impl Default for SshSettings {
    /// SSH off, port 22, no user, key sign-in: what a device gets when the operator gives none.
    fn default() -> Self {
        Self {
            enabled: false,
            port: 22,
            username: String::new(),
            auth: SshAuth::PrivateKey,
        }
    }
}

/// A group of devices on one subnet (VPN-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub id: SiteId,
    pub name: String,
    pub subnet: IpNet,
    /// The device that sends Wake-on-LAN packets at this site (D-7).
    pub(super) wake_relay: Option<DeviceId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub id: DeviceId,
    pub site: SiteId,
    pub name: String,
    pub host: String,
    pub ip: Option<IpAddr>,
    pub kind: String,
    pub os: OperatingSystem,
    pub tags: Vec<String>,
    pub maintenance: bool,
    pub ssh: SshSettings,
    pub rdp: RdpSettings,
    pub vnc: VncSettings,
    pub mac_address: Option<MacAddress>,
    pub metrics_source: MetricsSource,
    /// Most sessions open at once on this device; `None` is no limit (RA-036).
    pub session_limit: Option<NonZeroU8>,
    pub transfer: TransferPolicy,
    /// Read over SSH, never entered; `None` until read (DEV-016).
    pub distribution: Option<Distribution>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A validated site to create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NewSite {
    pub name: String,
    pub subnet: Cidr,
}

/// A validated device to create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NewDevice {
    pub site: SiteId,
    pub name: String,
    pub host: String,
    pub kind: String,
    pub os: OperatingSystem,
    pub tags: Vec<String>,
    pub ssh: SshSettings,
    pub remote: RemoteAccess,
}

/// A validated partial update: `None` leaves the field as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DeviceChange {
    pub site: Option<SiteId>,
    pub name: Option<String>,
    pub host: Option<String>,
    pub kind: Option<String>,
    pub os: Option<OperatingSystem>,
    pub tags: Option<Vec<String>>,
    pub maintenance: Option<bool>,
    pub ssh: Option<SshSettings>,
    pub remote: RemoteChange,
}

impl NewDevice {
    /// The host as an address when it is an IP literal (R-026).
    pub(super) fn ip(&self) -> Option<IpAddr> {
        ip_literal(&self.host)
    }
}

impl DeviceChange {
    /// The address the new host spells out; `None` for a name or when the host is not set.
    /// The store writes it only when the host is set; otherwise WP-09 owns `ip` (R-026).
    pub(super) fn ip(&self) -> Option<IpAddr> {
        self.host.as_deref().and_then(ip_literal)
    }

    /// Whether the update changes nothing.
    pub(super) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Which devices a list returns; every part that is set must match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DeviceFilter {
    /// Case-insensitive substring of the name.
    pub name: Option<String>,
    /// Devices carrying all of these tags.
    pub tags: Vec<String>,
    pub site: Option<SiteId>,
}

/// SSH settings as typed in the form; `port` is the raw GraphQL integer.
#[derive(Clone, Debug)]
pub(super) struct SshInput {
    pub(super) enabled: bool,
    pub(super) port: Option<i32>,
    pub(super) username: String,
    pub(super) auth: SshAuth,
}

/// A device to create, as typed in the form.
#[derive(Clone, Debug)]
pub(super) struct NewDeviceInput {
    pub(super) site: SiteId,
    pub(super) name: String,
    pub(super) host: String,
    pub(super) kind: String,
    pub(super) os: OperatingSystem,
    pub(super) tags: Vec<String>,
    pub(super) ssh: Option<SshInput>,
    pub(super) remote: RemoteForm,
}

/// A device update as typed in the form; absent fields stay as they are.
#[derive(Clone, Debug, Default)]
pub(super) struct DeviceChangeInput {
    pub(super) site: Option<SiteId>,
    pub(super) name: Option<String>,
    pub(super) host: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) os: Option<OperatingSystem>,
    pub(super) tags: Option<Vec<String>>,
    pub(super) maintenance: Option<bool>,
    pub(super) ssh: Option<SshInput>,
    pub(super) remote: RemoteForm,
}

/// A site to create, as typed in the form.
#[derive(Clone, Debug)]
pub(super) struct NewSiteInput {
    pub(super) name: String,
    pub(super) subnet: Cidr,
}
