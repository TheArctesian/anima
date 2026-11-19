//! The OwnTracks JSON of one HTTP post (K-E4 Section 11), built from the documents of
//! OwnTracks; no code of it is copied. Pure.

use chrono::DateTime;
use serde::Deserialize;

use super::model::{NewSample, Position};

/// Kilometres in an hour for each metre in a second.
const KMH_PER_MPS: f64 = 3.6;

/// The body is not an OwnTracks message, or its location has no usable time or position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the body is not an OwnTracks message")]
pub(super) struct Malformed;

#[derive(Deserialize)]
struct Message {
    #[serde(rename = "_type")]
    kind: String,
    tst: Option<i64>,
    lat: Option<f64>,
    lon: Option<f64>,
    acc: Option<f64>,
    alt: Option<f64>,
    vel: Option<f64>,
    tid: Option<String>,
}

/// The sample of one message: `Some` for `_type: location`, `None` for every other type,
/// which the hub accepts and does not store. The key is `<tid>:<tst>`.
///
/// # Errors
/// [`Malformed`] for a body that is not a JSON object with `_type`, and for a location with
/// no `tst`, `lat` or `lon`.
pub(super) fn sample(body: &[u8]) -> Result<Option<NewSample>, Malformed> {
    let message: Message = serde_json::from_slice(body).map_err(|_| Malformed)?;
    if message.kind != "location" {
        return Ok(None);
    }
    let (Some(tst), Some(latitude), Some(longitude)) = (message.tst, message.lat, message.lon)
    else {
        return Err(Malformed);
    };
    let observed_at = DateTime::from_timestamp(tst, 0).ok_or(Malformed)?;
    Ok(Some(NewSample {
        key: format!("{}:{tst}", message.tid.as_deref().unwrap_or_default()),
        observed_at: Some(observed_at),
        position: Position {
            latitude: Some(latitude),
            longitude: Some(longitude),
            accuracy_m: message.acc,
            altitude_m: message.alt,
            speed_mps: message.vel.map(|kmh| kmh / KMH_PER_MPS),
        },
    }))
}
