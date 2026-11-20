//! One point of Dawarich, as the hub keeps it.

use serde_json::Value;

/// Kilometres in an hour for each metre in a second.
const KMH_PER_MPS: f64 = 3.6;

/// One point. Each number is `None` when Dawarich gave none: no number takes its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// The id of the point at Dawarich, as text.
    pub id: String,
    /// The time of the fix, in Unix seconds.
    pub timestamp: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub accuracy_m: Option<f64>,
    pub altitude_m: Option<f64>,
    pub speed_mps: Option<f64>,
}

/// A number that Dawarich writes as a number or as text.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
    .filter(|number: &f64| number.is_finite())
}

impl Point {
    /// The point of one entry of a page; `None` when it has no `id`.
    #[must_use]
    pub fn read(entry: &Value) -> Option<Self> {
        let id = match &entry["id"] {
            Value::Number(id) => id.to_string(),
            Value::String(id) if !id.trim().is_empty() => id.trim().to_owned(),
            _ => return None,
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a Unix time in seconds is far inside the range of i64"
        )]
        let timestamp = number(&entry["timestamp"]).map(|seconds| seconds.trunc() as i64);
        Some(Self {
            id,
            timestamp,
            latitude: number(&entry["latitude"]),
            longitude: number(&entry["longitude"]),
            accuracy_m: number(&entry["accuracy"]),
            altitude_m: number(&entry["altitude"]),
            speed_mps: number(&entry["velocity"]).map(|kmh| kmh / KMH_PER_MPS),
        })
    }
}
