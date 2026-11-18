//! What the link answers: its devices, a new invite, a revocation and its refusals, in the
//! words of `link/cmd/anima-link/hubverbs.go` and `inviteverbs.go`.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use std::{fmt, net::IpAddr};

use async_graphql::Enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use zeroize::Zeroizing;

/// How a device of the link reaches the hub now (LNK-006).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Enum)]
#[serde(rename_all = "lowercase")]
pub enum LinkPath {
    Direct,
    Relayed,
    /// No packet has crossed yet.
    None,
}

/// One device of the `devices.list` reply. Unknown fields are ignored: the link may add some.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct ListedDevice {
    /// The device's public key, in the text form the link reads back in `devices.revoke`.
    pub key: String,
    pub fingerprint: String,
    pub name: String,
    pub address: IpAddr,
    pub expires: DateTime<Utc>,
    pub streams: i64,
    pub path: LinkPath,
    #[serde(default)]
    pub relay: Option<String>,
    /// When the device last said that it is connected (HMD-047). `None` for a device that did
    /// not say so since the link started, and for a link older than this field.
    #[serde(default)]
    pub seen: Option<DateTime<Utc>>,
}

/// An invite: a secret until it is used or expires. It has no `Display`, its `Debug` prints no
/// invite, and its memory is zeroed when it drops (LOG-007).
pub(super) struct Invite(Zeroizing<String>);

impl Invite {
    /// The invite, for the one reply that hands it to the operator.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Invite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[link invite]")
    }
}

impl<'de> Deserialize<'de> for Invite {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|invite| Self(Zeroizing::new(invite)))
    }
}

/// The `invites.make` reply.
#[derive(Debug, Deserialize)]
pub(super) struct MadeInvite {
    pub invite: Invite,
    pub expires: DateTime<Utc>,
}

/// One open invite of the `invites.list` reply: no invite text, only the fingerprint of its
/// temporary key, which `invites.cancel` takes.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(super) struct ListedInvite {
    pub id: String,
    pub name: String,
    pub expires: DateTime<Utc>,
}

/// The `devices.revoke` reply.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Revoked {
    pub streams_closed: i64,
}

/// Why the link refused a request, by the code it sent. The link's own message is not kept:
/// the hub says it in its own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `device_unknown`: no permitted device has this key.
    DeviceUnknown,
    /// `bad_name`: a device needs a name of 1 to 64 visible characters.
    BadName,
    /// `bad_request`: the arguments are not valid (for example, a key that is not a key).
    BadRequest,
    /// Any other code: a fault of the link or of the hub.
    Other,
}

impl Refusal {
    /// The refusal a code of the link names.
    pub(super) fn of_code(code: &str) -> Self {
        match code {
            "device_unknown" => Self::DeviceUnknown,
            "bad_name" => Self::BadName,
            "bad_request" => Self::BadRequest,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DeviceUnknown => "device_unknown",
            Self::BadName => "bad_name",
            Self::BadRequest => "bad_request",
            Self::Other => "other",
        })
    }
}
