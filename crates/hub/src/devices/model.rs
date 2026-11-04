//! Domain types of the device inventory (DEV-010).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{net::IpAddr, str::FromStr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use ipnet::IpNet;

use crate::ids::{DeviceId, SiteId};

/// The database holds text that is not a known value of an enum.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} '{text}'")]
pub struct UnknownValue {
    kind: &'static str,
    text: String,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A validated site to create.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices store"))]
pub(super) struct NewSite {
    pub name: String,
    pub subnet: IpNet,
}

/// A validated device to create.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices store"))]
pub(super) struct NewDevice {
    pub site: SiteId,
    pub name: String,
    pub host: String,
    pub kind: String,
    pub os: OperatingSystem,
    pub tags: Vec<String>,
    pub ssh: SshSettings,
}

/// A validated partial update: `None` leaves the field as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(test), expect(dead_code, reason = "used by the devices store"))]
pub(super) struct DeviceChange {
    pub site: Option<SiteId>,
    pub name: Option<String>,
    pub host: Option<String>,
    pub kind: Option<String>,
    pub os: Option<OperatingSystem>,
    pub tags: Option<Vec<String>>,
    pub maintenance: Option<bool>,
    pub ssh: Option<SshSettings>,
}

/// Which devices a list returns; every part that is set must match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(dead_code, reason = "used by the devices store")]
pub(super) struct DeviceFilter {
    /// Case-insensitive substring of the name.
    pub name: Option<String>,
    /// Devices carrying all of these tags.
    pub tags: Vec<String>,
    pub site: Option<SiteId>,
}
