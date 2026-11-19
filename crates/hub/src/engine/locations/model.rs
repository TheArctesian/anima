//! What a location sample and an ingest token are (K-E4 Section 11). Pure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{IngestTokenId, SampleId};

/// The longest account or sample key, in characters.
pub(super) const MAX_KEY: usize = 200;
/// The longest name of an ingest token, in characters.
pub(super) const MAX_TOKEN_NAME: usize = 64;

/// Where a sample came from. The identity of the provider stays with the sample (R-145).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Dawarich,
    OwnTracks,
    /// The Anima phone apps.
    Anima,
}

impl Provider {
    /// The text stored in `locations.sample.provider`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dawarich => "dawarich",
            Self::OwnTracks => "owntracks",
            Self::Anima => "anima",
        }
    }

    /// The provider whose stored text is `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Dawarich, Self::OwnTracks, Self::Anima]
            .into_iter()
            .find(|provider| provider.as_str() == text)
    }
}

/// The provider and the account or ingest token that gave a batch of samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub provider: Provider,
    pub account: String,
}

/// The sealed part of a sample. Each number is `None` when the provider gave none: no code
/// puts a number in the place of a missing one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub accuracy_m: Option<f64>,
    pub altitude_m: Option<f64>,
    pub speed_mps: Option<f64>,
}

/// One sample as a provider gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct NewSample {
    /// The identity that the provider gives the sample.
    pub key: String,
    /// The time of the fix; `None` when the provider gave none.
    pub observed_at: Option<DateTime<Utc>>,
    pub position: Position,
}

/// One stored sample, opened.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub id: SampleId,
    pub provider: Provider,
    pub account: String,
    pub key: String,
    pub observed_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub position: Position,
}

/// How many samples of a batch were new and how many the hub already had.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    pub added: u64,
    pub known: u64,
}

/// One ingest token of the OwnTracks path, without its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestToken {
    pub id: IngestTokenId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// One input field to fix, with what to give.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bad {
    pub field: String,
    pub message: &'static str,
}

impl Bad {
    pub(super) fn new(field: impl Into<String>, message: &'static str) -> Self {
        Self {
            field: field.into(),
            message,
        }
    }
}

fn one_line(text: &str, most: usize) -> Option<String> {
    let text = text.trim();
    let valid =
        !text.is_empty() && text.chars().count() <= most && !text.chars().any(char::is_control);
    valid.then(|| text.to_owned())
}

/// The name of an ingest token: one line of 1 to 64 characters.
pub(super) fn token_name(text: &str) -> Option<String> {
    one_line(text, MAX_TOKEN_NAME)
}

fn within(value: Option<f64>, low: f64, high: f64) -> bool {
    value.is_none_or(|number| number.is_finite() && (low..=high).contains(&number))
}

/// Each field of the sample at `index` that is not valid. A sample gives a latitude and a
/// longitude together or gives neither.
pub(super) fn faults(index: usize, sample: &NewSample) -> Vec<Bad> {
    let position = sample.position;
    let field = |name: &str| format!("samples[{index}].{name}");
    let mut bad = Vec::new();
    if one_line(&sample.key, MAX_KEY).as_deref() != Some(sample.key.as_str()) {
        bad.push(Bad::new(
            field("key"),
            "Give one line of 1 to 200 characters.",
        ));
    }
    if position.latitude.is_some() != position.longitude.is_some() {
        bad.push(Bad::new(
            field("latitude"),
            "Give latitude and longitude together.",
        ));
    }
    if !within(position.latitude, -90.0, 90.0) {
        bad.push(Bad::new(
            field("latitude"),
            "Give a latitude from -90 to 90.",
        ));
    }
    if !within(position.longitude, -180.0, 180.0) {
        bad.push(Bad::new(
            field("longitude"),
            "Give a longitude from -180 to 180.",
        ));
    }
    if !within(position.accuracy_m, 0.0, f64::MAX) {
        bad.push(Bad::new(
            field("accuracyM"),
            "Give an accuracy of 0 or more metres.",
        ));
    }
    if !within(position.altitude_m, f64::MIN, f64::MAX) {
        bad.push(Bad::new(field("altitudeM"), "Give a number of metres."));
    }
    if !within(position.speed_mps, 0.0, f64::MAX) {
        bad.push(Bad::new(
            field("speedMps"),
            "Give a speed of 0 or more metres in a second.",
        ));
    }
    bad
}
