//! The SNMP check (D-5): the stored part, free of secrets, and the secrets an operator
//! supplies, which are sealed apart from it (SEC-003, LOG-007).
//!
//! Secret types here implement neither `Debug` nor `Display`.

use serde::{Deserialize, Serialize};

use super::rules::{self, ConfigError, Reason, err};
use crate::fleet::credentials::Secret;

/// The SNMP versions the hub speaks: v2c with a community, v3 with authPriv only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SnmpVersion {
    #[default]
    V2c,
    V3,
}

const fn port() -> u16 {
    161
}
const fn timeout_ms() -> u32 {
    5000
}

/// The longest expected value, in bytes.
pub const EXPECTED_MAX: usize = 1024;
/// The longest v3 user name, in bytes (RFC 3414 `usmUserName`).
pub const USERNAME_MAX: usize = 32;
/// The longest community or password, in bytes.
pub const SECRET_MAX: usize = 256;
/// The fewest characters of a v3 password (RFC 3414 Section 11.2).
pub const PASSWORD_MIN: usize = 8;
/// The most arcs an OID has (RFC 2578 Section 3.5).
pub const OID_ARCS_MAX: usize = 128;

/// The stored, secret-free part of an SNMP check. `username` is the v3 user; v2c has none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnmpConfig {
    #[serde(default = "port")]
    pub port: u16,
    #[serde(default)]
    pub version: SnmpVersion,
    pub oid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default = "timeout_ms")]
    pub timeout_ms: u32,
}

/// The arcs of a dotted OID such as `1.3.6.1.2.1.1.3.0`: at least two, each a `u32`, the
/// first at most 2 and the second below 40 unless the first is 2, so the BER encoding
/// holds every one. `None` for anything else.
#[must_use]
pub fn oid_arcs(text: &str) -> Option<Vec<u32>> {
    let mut arcs = Vec::new();
    for part in text.split('.') {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        arcs.push(part.parse::<u32>().ok()?);
    }
    match arcs.as_slice() {
        [first, second, ..] if *first < 2 && *second < 40 => {}
        [2, _, ..] => {}
        _ => return None,
    }
    (arcs.len() <= OID_ARCS_MAX).then_some(arcs)
}

impl SnmpConfig {
    /// Checks every field; the error names the field and never repeats its value.
    ///
    /// # Errors
    /// The first refused field.
    pub fn validate(&self) -> Result<(), ConfigError> {
        rules::port("port", self.port)?;
        if self.oid.is_empty() {
            return Err(err("oid", Reason::Empty));
        }
        if oid_arcs(&self.oid).is_none() {
            return Err(err("oid", Reason::Invalid));
        }
        if let Some(expected) = &self.expected {
            rules::max_len("expected", expected, EXPECTED_MAX)?;
        }
        match (self.version, self.username.as_deref()) {
            (SnmpVersion::V2c, None) => {}
            (SnmpVersion::V2c, Some(_)) => return Err(err("username", Reason::OtherVersion)),
            (SnmpVersion::V3, None) => return Err(err("username", Reason::Empty)),
            (SnmpVersion::V3, Some(name)) => username(name)?,
        }
        rules::timeout("timeoutMs", self.timeout_ms)
    }
}

/// A v3 user name: 1 to 32 bytes, no control characters.
fn username(name: &str) -> Result<(), ConfigError> {
    if name.is_empty() {
        return Err(err("username", Reason::Empty));
    }
    rules::max_len("username", name, USERNAME_MAX)?;
    if name.chars().any(char::is_control) {
        return Err(err("username", Reason::ControlCharacter));
    }
    Ok(())
}

/// The secrets of an SNMP check, for the version they were given for.
///
/// Like [`Secret`] it has no `Debug`, `Display`, `Clone` or `Serialize`.
pub enum SnmpSecrets {
    /// v2c.
    Community(Secret),
    /// v3 authPriv: the authentication and the privacy password.
    User { auth: Secret, privacy: Secret },
}

impl SnmpSecrets {
    /// The version these secrets are for.
    #[must_use]
    pub const fn version(&self) -> SnmpVersion {
        match self {
            Self::Community(_) => SnmpVersion::V2c,
            Self::User { .. } => SnmpVersion::V3,
        }
    }
}

/// The secrets as the operator entered them; `None` (or an empty value) asks to keep the
/// stored one.
#[derive(Default)]
pub struct SnmpCredentials {
    pub community: Option<Secret>,
    pub auth_password: Option<Secret>,
    pub privacy_password: Option<Secret>,
}

/// An SNMP check as an operator supplies it, secrets in hand. It prints nothing (it has no
/// `Debug`); [`SnmpInput::split`] separates the stored part from the secrets.
pub struct SnmpInput {
    pub config: SnmpConfig,
    pub credentials: SnmpCredentials,
}

/// `None` for a secret that is absent or empty: both keep the stored one.
fn given(secret: Option<Secret>) -> Option<Secret> {
    secret.filter(|secret| !secret.expose().is_empty())
}

/// A community: at most 256 bytes.
fn community(secret: &Secret) -> Result<(), ConfigError> {
    if secret.expose().len() > SECRET_MAX {
        return Err(err("community", Reason::TooLong { max: SECRET_MAX }));
    }
    Ok(())
}

/// A v3 password: UTF-8, at least 8 characters and at most 256 bytes.
fn password(field: &'static str, secret: &Secret) -> Result<(), ConfigError> {
    let text = std::str::from_utf8(secret.expose()).map_err(|_| err(field, Reason::Invalid))?;
    if text.len() > SECRET_MAX {
        return Err(err(field, Reason::TooLong { max: SECRET_MAX }));
    }
    if text.chars().count() < PASSWORD_MIN {
        return Err(err(field, Reason::TooShort { min: PASSWORD_MIN }));
    }
    Ok(())
}

impl SnmpInput {
    /// Validates the configuration and the secrets given, and separates them. A secret of
    /// the other version is refused; it would be dropped otherwise.
    ///
    /// # Errors
    /// The first refused field; the error never repeats the value.
    pub fn split(self) -> Result<(SnmpConfig, SnmpCredentials), ConfigError> {
        self.config.validate()?;
        let credentials = SnmpCredentials {
            community: given(self.credentials.community),
            auth_password: given(self.credentials.auth_password),
            privacy_password: given(self.credentials.privacy_password),
        };
        match self.config.version {
            SnmpVersion::V2c => {
                if credentials.auth_password.is_some() {
                    return Err(err("authPassword", Reason::OtherVersion));
                }
                if credentials.privacy_password.is_some() {
                    return Err(err("privacyPassword", Reason::OtherVersion));
                }
                credentials.community.as_ref().map_or(Ok(()), community)?;
            }
            SnmpVersion::V3 => {
                if credentials.community.is_some() {
                    return Err(err("community", Reason::OtherVersion));
                }
                let auth = credentials.auth_password.as_ref();
                auth.map_or(Ok(()), |s| password("authPassword", s))?;
                let privacy = credentials.privacy_password.as_ref();
                privacy.map_or(Ok(()), |s| password("privacyPassword", s))?;
            }
        }
        Ok((self.config, credentials))
    }
}
