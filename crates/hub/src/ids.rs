//! Entity identifiers: one UUID newtype per entity, so a `DeviceId` never passes for a `SiteId`.
//!
//! Domains generate ids at their I/O edge; this module only wraps, parses and prints them.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The text is not a UUID.
#[derive(Debug, thiserror::Error)]
#[error("not a valid id")]
pub struct ParseIdError(#[from] uuid::Error);

macro_rules! entity_id {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
        #[sqlx(transparent)]
        pub struct $name(Uuid);

        impl FromStr for $name {
            type Err = ParseIdError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(text)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(f)
            }
        }
    )+};
}

entity_id! {
    /// An operator account.
    OperatorId,
    /// A site (a group of devices).
    SiteId,
    /// A managed device.
    DeviceId,
    /// A health check on a device.
    CheckId,
    /// A maintenance window.
    WindowId,
    /// A remote session.
    SessionId,
    /// An entry in the event log.
    EventId,
    /// A web interface of a device.
    WebInterfaceId,
    /// A metric series of a device.
    MetricSeriesId,
    /// An API key.
    ApiKeyId,
    /// A notification channel.
    ChannelId,
    /// A notification delivery.
    DeliveryId,
    /// A session share link.
    ShareId,
    /// A session recording.
    RecordingId,
    /// An access token of the operator or of a builder.
    TokenId,
    /// A release: one version and commit, built for one or more targets.
    ReleaseId,
    /// The build of one release for one target.
    BuildJobId,
    /// A computer the operator enrolled to build releases.
    BuilderId,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d";

    #[test]
    fn id_text_round_trips_through_from_str_and_display() {
        let id: DeviceId = TEXT.parse().unwrap();
        assert_eq!(id.to_string(), TEXT);
    }

    #[test]
    fn id_converts_to_and_from_a_generated_uuid() {
        let uuid = Uuid::now_v7();
        let id = EventId::from(uuid);
        assert_eq!(Uuid::from(id), uuid);
        assert_eq!(id.to_string(), uuid.hyphenated().to_string());
    }

    #[test]
    fn id_rejects_text_that_is_not_a_uuid() {
        assert!("not-an-id".parse::<SiteId>().is_err());
    }

    #[test]
    fn id_serialises_as_the_uuid_string() {
        let id: CheckId = TEXT.parse().unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), format!("\"{TEXT}\""));
    }
}
