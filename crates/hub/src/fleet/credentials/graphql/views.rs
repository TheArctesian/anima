//! The K1 types of the operational keys (K-E5 Section 2.2). No field holds a value: the value
//! goes in through an argument and never comes out (CLI-004, SEC-005, LOG-007).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject, SimpleObject};
use chrono::{DateTime, Utc};

use crate::fleet::credentials::operational::{
    Consumer, Due, Ending, Metadata, OperationalKey, Version, VersionState,
};

/// K1 `OperationalKeyDue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "OperationalKeyDue")]
pub enum DueView {
    Current,
    Due,
    Expiring,
    Expired,
}

/// K1 `OperationalKeyVersionState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
#[graphql(name = "OperationalKeyVersionState")]
pub enum StateView {
    Active,
    Superseded,
    Revoked,
    Expired,
}

/// K1 `OperationalKey`.
#[derive(SimpleObject)]
#[graphql(name = "OperationalKey")]
pub struct KeyView {
    id: ID,
    name: String,
    provider: String,
    purpose: String,
    instructions: String,
    provider_link: Option<String>,
    rotation_days: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
    due: DueView,
    active_version: Option<i32>,
    last_rotated_at: DateTime<Utc>,
    last_rotated_by: String,
    created_at: DateTime<Utc>,
    created_by: String,
    versions: Vec<VersionView>,
    consumers: Vec<ConsumerView>,
}

/// K1 `OperationalKeyVersion`.
#[derive(SimpleObject)]
#[graphql(name = "OperationalKeyVersion")]
pub struct VersionView {
    number: i32,
    state: StateView,
    created_at: DateTime<Utc>,
    created_by: String,
    ended_at: Option<DateTime<Utc>>,
    rotated_at_provider: bool,
}

/// K1 `OperationalKeyConsumer`.
#[derive(SimpleObject)]
#[graphql(name = "OperationalKeyConsumer")]
pub struct ConsumerView {
    device_id: ID,
    workload: String,
    granted_at: DateTime<Utc>,
    granted_by: String,
    held_version: Option<i32>,
    held_seen_at: Option<DateTime<Utc>>,
    fetched_version: Option<i32>,
    fetched_at: Option<DateTime<Utc>>,
}

/// K1 `OperationalKeyEnding`.
#[derive(SimpleObject)]
#[graphql(name = "OperationalKeyEnding")]
pub struct EndingView {
    key: KeyView,
    ended_version: Option<i32>,
    still_held_by: Vec<ConsumerView>,
}

/// K1 `CreateOperationalKeyInput`. No `Debug`: `value` is a secret.
#[derive(InputObject)]
#[graphql(name = "CreateOperationalKeyInput")]
pub struct CreateInput {
    pub name: String,
    pub provider: String,
    pub purpose: String,
    #[graphql(default)]
    pub instructions: String,
    pub provider_link: Option<String>,
    pub rotation_days: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub value: String,
}

/// K1 `UpdateOperationalKeyInput`: the whole metadata, which replaces the stored one.
#[derive(InputObject)]
#[graphql(name = "UpdateOperationalKeyInput")]
pub struct UpdateInput {
    pub provider: String,
    pub purpose: String,
    pub instructions: String,
    pub provider_link: Option<String>,
    pub rotation_days: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl CreateInput {
    /// The metadata of the input; its name and value are taken apart by the caller.
    pub fn metadata(&self) -> Metadata {
        Metadata {
            provider: self.provider.clone(),
            purpose: self.purpose.clone(),
            instructions: self.instructions.clone(),
            provider_link: self.provider_link.clone(),
            rotation_days: self.rotation_days,
            expires_at: self.expires_at,
        }
    }
}

impl From<UpdateInput> for Metadata {
    fn from(input: UpdateInput) -> Self {
        Self {
            provider: input.provider,
            purpose: input.purpose,
            instructions: input.instructions,
            provider_link: input.provider_link,
            rotation_days: input.rotation_days,
            expires_at: input.expires_at,
        }
    }
}

impl From<Due> for DueView {
    fn from(due: Due) -> Self {
        match due {
            Due::Current => Self::Current,
            Due::Due => Self::Due,
            Due::Expiring => Self::Expiring,
            Due::Expired => Self::Expired,
        }
    }
}

impl From<VersionState> for StateView {
    fn from(state: VersionState) -> Self {
        match state {
            VersionState::Active => Self::Active,
            VersionState::Superseded => Self::Superseded,
            VersionState::Revoked => Self::Revoked,
            VersionState::Expired => Self::Expired,
        }
    }
}

impl From<OperationalKey> for KeyView {
    fn from(key: OperationalKey) -> Self {
        let active_version = key.active_version();
        let newest = key.versions.first();
        let last_rotated_at = newest.map_or(key.created_at, |version| version.created_at);
        let last_rotated_by = newest.map_or_else(
            || key.created_by.clone(),
            |version| version.created_by.clone(),
        );
        Self {
            id: ID(key.id.to_string()),
            name: key.name,
            provider: key.metadata.provider,
            purpose: key.metadata.purpose,
            instructions: key.metadata.instructions,
            provider_link: key.metadata.provider_link,
            rotation_days: key.metadata.rotation_days,
            expires_at: key.metadata.expires_at,
            due: key.due.into(),
            active_version,
            last_rotated_at,
            last_rotated_by,
            created_at: key.created_at,
            created_by: key.created_by,
            versions: key.versions.into_iter().map(Into::into).collect(),
            consumers: key.consumers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Version> for VersionView {
    fn from(version: Version) -> Self {
        Self {
            number: version.number,
            state: version.state.into(),
            created_at: version.created_at,
            created_by: version.created_by,
            ended_at: version.ended_at,
            rotated_at_provider: version.rotated_at_provider,
        }
    }
}

impl From<Consumer> for ConsumerView {
    fn from(consumer: Consumer) -> Self {
        Self {
            device_id: ID(consumer.device.to_string()),
            workload: consumer.workload,
            granted_at: consumer.granted_at,
            granted_by: consumer.granted_by,
            held_version: consumer.held_version,
            held_seen_at: consumer.held_seen_at,
            fetched_version: consumer.fetched_version,
            fetched_at: consumer.fetched_at,
        }
    }
}

impl From<Ending> for EndingView {
    fn from(ending: Ending) -> Self {
        Self {
            key: ending.key.into(),
            ended_version: ending.ended,
            still_held_by: ending.still_held_by.into_iter().map(Into::into).collect(),
        }
    }
}
