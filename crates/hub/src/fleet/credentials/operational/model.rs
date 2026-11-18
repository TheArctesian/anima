//! What an operational key is, as the hub keeps and shows it (K-E5 Section 2). No type here
//! holds a value: the value is a [`Secret`](crate::fleet::credentials::Secret) until it is sealed.

use chrono::{DateTime, Utc};

use crate::ids::{DeviceId, OperationalKeyId};

/// What the operator says about a key: everything but its name and its value (LOG-007: no
/// field holds a value).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub provider: String,
    pub purpose: String,
    /// How to rotate the key at its provider, at most 4 000 characters.
    pub instructions: String,
    /// An `https` address at the provider.
    pub provider_link: Option<String>,
    pub rotation_days: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// The state of one version (K-E5 Section 2). A version that ended is never active again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionState {
    Active,
    Superseded,
    Revoked,
    Expired,
}

impl VersionState {
    /// The text stored in `operational_key_versions.state`.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    /// The state whose stored text is `text`; `None` for text no version holds.
    #[must_use]
    pub fn from_sql(text: &str) -> Option<Self> {
        [Self::Active, Self::Superseded, Self::Revoked, Self::Expired]
            .into_iter()
            .find(|state| state.as_sql() == text)
    }
}

/// When the key needs the operator (K-E5 Section 2): `Due` when the last rotation is older than
/// `rotation_days`, `Expiring` in the 14 days before `expires_at`, `Expired` after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Due {
    Current,
    Due,
    Expiring,
    Expired,
}

/// One version of a key, as it reads now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub number: i32,
    pub state: VersionState,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub ended_at: Option<DateTime<Utc>>,
    /// What the operator said: the key was also changed at its provider.
    pub rotated_at_provider: bool,
}

/// A server that may fetch a key: its device and the workload that uses the key there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consumer {
    pub device: DeviceId,
    pub workload: String,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
    /// The version in the daemon's cache at its last report.
    pub held_version: Option<i32>,
    pub held_seen_at: Option<DateTime<Utc>>,
    pub fetched_version: Option<i32>,
    pub fetched_at: Option<DateTime<Utc>>,
}

/// A key with its versions (newest first), its consumers and its due state at the time it was
/// read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationalKey {
    pub id: OperationalKeyId,
    pub name: String,
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub versions: Vec<Version>,
    pub consumers: Vec<Consumer>,
    pub due: Due,
}

impl OperationalKey {
    /// The number of the active version; `None` when no version is active.
    #[must_use]
    pub fn active_version(&self) -> Option<i32> {
        self.versions
            .iter()
            .find(|version| version.state == VersionState::Active)
            .map(|version| version.number)
    }
}

/// What a change that ends a version leaves: the key after it, the version it ended, and the
/// consumers whose cache held that version at their last report (K-E5 Section 2.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ending {
    pub key: OperationalKey,
    pub ended: Option<i32>,
    pub still_held_by: Vec<Consumer>,
}

/// A grant: the key, the device and the workload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub name: String,
    pub device: DeviceId,
    pub workload: String,
}
