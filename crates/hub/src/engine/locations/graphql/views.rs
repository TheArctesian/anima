//! The K1 types of the locations part.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject, SimpleObject};
use chrono::{DateTime, Utc};

use crate::engine::locations::{
    ActorKind, Basis, Direction, End, IngestToken, Issued, NewSample, OperatorEvidence, Position,
    Provider, Sample, StoredContext,
};

/// K1 `LocationProvider`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "LocationProvider")]
pub(super) enum ProviderView {
    Dawarich,
    #[graphql(name = "OWNTRACKS")]
    OwnTracks,
    Anima,
}

/// K1 `LocationSample`. Each number is null when the provider gave none.
#[derive(SimpleObject)]
#[graphql(name = "LocationSample")]
pub(super) struct SampleView {
    id: ID,
    provider: ProviderView,
    account: String,
    key: String,
    observed_at: Option<DateTime<Utc>>,
    received_at: DateTime<Utc>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    accuracy_m: Option<f64>,
    altitude_m: Option<f64>,
    speed_mps: Option<f64>,
}

impl From<Sample> for SampleView {
    fn from(sample: Sample) -> Self {
        Self {
            id: ID(sample.id.to_string()),
            provider: match sample.provider {
                Provider::Dawarich => ProviderView::Dawarich,
                Provider::OwnTracks => ProviderView::OwnTracks,
                Provider::Anima => ProviderView::Anima,
            },
            account: sample.account,
            key: sample.key,
            observed_at: sample.observed_at,
            received_at: sample.received_at,
            latitude: sample.position.latitude,
            longitude: sample.position.longitude,
            accuracy_m: sample.position.accuracy_m,
            altitude_m: sample.position.altitude_m,
            speed_mps: sample.position.speed_mps,
        }
    }
}

/// K1 `LocationSampleInput`: one sample of an Anima phone app.
#[derive(InputObject)]
pub(super) struct LocationSampleInput {
    /// The identifier that the app made for the sample.
    key: String,
    observed_at: Option<DateTime<Utc>>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    accuracy_m: Option<f64>,
    altitude_m: Option<f64>,
    speed_mps: Option<f64>,
}

impl From<LocationSampleInput> for NewSample {
    fn from(input: LocationSampleInput) -> Self {
        Self {
            key: input.key,
            observed_at: input.observed_at,
            position: Position {
                latitude: input.latitude,
                longitude: input.longitude,
                accuracy_m: input.accuracy_m,
                altitude_m: input.altitude_m,
                speed_mps: input.speed_mps,
            },
        }
    }
}

/// K1 `LocationIngestToken`: never its value.
#[derive(SimpleObject)]
#[graphql(name = "LocationIngestToken")]
pub(super) struct TokenView {
    id: ID,
    name: String,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl From<IngestToken> for TokenView {
    fn from(token: IngestToken) -> Self {
        Self {
            id: ID(token.id.to_string()),
            name: token.name,
            created_at: token.created_at,
            last_used_at: token.last_used_at,
            revoked_at: token.revoked_at,
        }
    }
}

/// K1 `IssuedLocationIngestToken`: the token and its value, shown one time.
#[derive(SimpleObject)]
#[graphql(name = "IssuedLocationIngestToken")]
pub(super) struct IssuedTokenView {
    ingest_token: TokenView,
    token: String,
}

impl From<Issued> for IssuedTokenView {
    fn from(issued: Issued) -> Self {
        Self {
            token: issued.value.to_string(),
            ingest_token: issued.token.into(),
        }
    }
}

/// K1 `TransferDirection`, seen from the device.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "TransferDirection")]
enum DirectionView {
    Upload,
    Download,
}

/// K1 `TransferActorKind`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "TransferActorKind")]
enum ActorKindView {
    Human,
    Agent,
    System,
}

/// K1 `LocationEvidenceBasis`.
#[derive(Clone, Copy, PartialEq, Eq, Enum)]
#[graphql(name = "LocationEvidenceBasis")]
enum BasisView {
    AtTheTime,
    NearestInferred,
    Unknown,
}

/// K1 `TransferEnd`: one side of a transfer as it was at the event. Each ID is null when the
/// hub did not know it.
#[derive(SimpleObject)]
#[graphql(name = "TransferEnd")]
#[expect(
    clippy::struct_field_names,
    reason = "K1 names the three fields deviceId, siteId and placeId"
)]
struct EndView {
    device_id: Option<ID>,
    site_id: Option<ID>,
    place_id: Option<ID>,
}

impl From<End> for EndView {
    fn from(end: End) -> Self {
        Self {
            device_id: end.device.map(|id| ID(id.to_string())),
            site_id: end.site.map(|id| ID(id.to_string())),
            place_id: end.place.map(|id| ID(id.to_string())),
        }
    }
}

/// K1 `OperatorLocationEvidence`: where the operator was at the event, as far as the samples
/// say. `placeId` is given only for a sample at the time that is not uncertain.
#[derive(SimpleObject)]
#[graphql(name = "OperatorLocationEvidence")]
struct EvidenceView {
    basis: BasisView,
    sample_id: Option<ID>,
    offset_seconds: Option<i32>,
    uncertain: bool,
    place_id: Option<ID>,
}

impl From<OperatorEvidence> for EvidenceView {
    fn from(evidence: OperatorEvidence) -> Self {
        Self {
            basis: match evidence.basis {
                Basis::AtTheTime => BasisView::AtTheTime,
                Basis::NearestInferred => BasisView::NearestInferred,
                Basis::Unknown => BasisView::Unknown,
            },
            sample_id: evidence.sample.map(|id| ID(id.to_string())),
            offset_seconds: evidence
                .offset_seconds
                .and_then(|seconds| i32::try_from(seconds).ok()),
            uncertain: evidence.uncertain,
            place_id: evidence.place.map(|id| ID(id.to_string())),
        }
    }
}

/// K1 `TransferContext`.
#[derive(SimpleObject)]
#[graphql(name = "TransferContext")]
pub(super) struct TransferContextView {
    event_id: ID,
    occurred_at: DateTime<Utc>,
    direction: DirectionView,
    source: EndView,
    destination: EndView,
    actor_kind: ActorKindView,
    actor: Option<String>,
    client_device_id: Option<ID>,
    operator: EvidenceView,
}

impl From<(StoredContext, OperatorEvidence)> for TransferContextView {
    fn from((stored, evidence): (StoredContext, OperatorEvidence)) -> Self {
        Self {
            event_id: ID(stored.event.to_string()),
            occurred_at: stored.occurred_at,
            direction: match stored.direction {
                Direction::Upload => DirectionView::Upload,
                Direction::Download => DirectionView::Download,
            },
            source: stored.source.into(),
            destination: stored.destination.into(),
            actor_kind: match stored.actor.kind {
                ActorKind::Human => ActorKindView::Human,
                ActorKind::Agent => ActorKindView::Agent,
                ActorKind::System => ActorKindView::System,
            },
            actor: stored.actor.name,
            client_device_id: stored.actor.client_device.map(|id| ID(id.to_string())),
            operator: evidence.into(),
        }
    }
}
