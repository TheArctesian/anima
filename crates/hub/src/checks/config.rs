//! Typed check configuration and its stored JSON form (CHK-009).
//!
//! The `config` column holds [`CheckConfig`] as JSON with an internal tag:
//! `{"type":"tcp","port":22,"timeout_ms":5000}`. The tag values are the fixed type ids
//! (`icmp tcp tls http dns push group`) and never change meaning. Secrets are never part of
//! this JSON: see [`HttpSecrets`].

mod http;
mod rules;

use std::{net::IpAddr, time::Duration};

use serde::{Deserialize, Serialize};

pub use http::{
    HttpAuth, HttpConfig, HttpCredentials, HttpHeader, HttpInput, HttpMethod, HttpSecrets,
    StatusRange,
};
pub use rules::{ConfigError, Reason, is_secret_header};

/// The fixed database identifier of a check type (CHK-009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckType {
    Icmp,
    Tcp,
    Tls,
    Http,
    Dns,
    Push,
    Group,
}

impl CheckType {
    /// The id stored in the database. Never change an existing value.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Icmp => "icmp",
            Self::Tcp => "tcp",
            Self::Tls => "tls",
            Self::Http => "http",
            Self::Dns => "dns",
            Self::Push => "push",
            Self::Group => "group",
        }
    }

    #[must_use]
    pub fn from_sql(id: &str) -> Option<Self> {
        Some(match id {
            "icmp" => Self::Icmp,
            "tcp" => Self::Tcp,
            "tls" => Self::Tls,
            "http" => Self::Http,
            "dns" => Self::Dns,
            "push" => Self::Push,
            "group" => Self::Group,
            _ => return None,
        })
    }
}

/// Address family an ICMP check uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpVersion {
    #[default]
    V4,
    V6,
}

/// Record type a DNS check asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsRecordType {
    #[default]
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Txt,
    Srv,
    Ptr,
}

const fn icmp_timeout_ms() -> u32 {
    2000
}
const fn network_timeout_ms() -> u32 {
    5000
}
const fn tls_port() -> u16 {
    443
}

/// What a check does, with the settings of its type. Contains no secrets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckConfig {
    Icmp {
        #[serde(default = "icmp_timeout_ms")]
        timeout_ms: u32,
        #[serde(default)]
        ip_version: IpVersion,
    },
    Tcp {
        port: u16,
        #[serde(default = "network_timeout_ms")]
        timeout_ms: u32,
    },
    Tls {
        #[serde(default = "tls_port")]
        port: u16,
        #[serde(default)]
        server_name: Option<String>,
        #[serde(default = "network_timeout_ms")]
        timeout_ms: u32,
    },
    Http(HttpConfig),
    Dns {
        name: String,
        #[serde(default)]
        record_type: DnsRecordType,
        #[serde(default)]
        resolver: Option<IpAddr>,
        #[serde(default)]
        expected: Option<String>,
    },
    /// The device calls the hub; nothing is probed.
    Push,
    /// Status comes from the children; nothing is probed.
    Group,
}

impl CheckConfig {
    #[must_use]
    pub const fn kind(&self) -> CheckType {
        match self {
            Self::Icmp { .. } => CheckType::Icmp,
            Self::Tcp { .. } => CheckType::Tcp,
            Self::Tls { .. } => CheckType::Tls,
            Self::Http(_) => CheckType::Http,
            Self::Dns { .. } => CheckType::Dns,
            Self::Push => CheckType::Push,
            Self::Group => CheckType::Group,
        }
    }

    /// The time limit of one probe; DNS has none of its own and uses the network default.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        let ms = match self {
            Self::Icmp { timeout_ms, .. }
            | Self::Tcp { timeout_ms, .. }
            | Self::Tls { timeout_ms, .. } => *timeout_ms,
            Self::Http(http) => http.timeout_ms,
            Self::Dns { .. } | Self::Push | Self::Group => network_timeout_ms(),
        };
        Duration::from_millis(u64::from(ms))
    }

    /// Checks every field; the error names the field and never repeats its value.
    ///
    /// # Errors
    /// The first refused field.
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Self::Icmp { timeout_ms, .. } => rules::timeout("timeoutMs", *timeout_ms),
            Self::Tcp { port, timeout_ms } => {
                rules::port("port", *port)?;
                rules::timeout("timeoutMs", *timeout_ms)
            }
            Self::Tls {
                port,
                server_name,
                timeout_ms,
            } => {
                rules::port("port", *port)?;
                rules::timeout("timeoutMs", *timeout_ms)?;
                server_name
                    .as_deref()
                    .map_or(Ok(()), |name| rules::dns_name("serverName", name, false))
            }
            Self::Http(http) => http.validate(),
            Self::Dns { name, expected, .. } => {
                rules::dns_name("name", name, true)?;
                expected
                    .as_deref()
                    .map_or(Ok(()), |text| rules::max_len("expected", text, 1024))
            }
            Self::Push | Self::Group => Ok(()),
        }
    }
}
