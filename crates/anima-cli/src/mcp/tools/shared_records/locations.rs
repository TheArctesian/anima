//! The location tools of the `records` set (K-E4 Sections 11 to 13; R-145). A position is
//! Class B data of the operator. No tool makes or returns an ingest token: a token is a secret.

use rmcp::model::JsonObject;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::hub::Hub;
use crate::locations::documents::{
    LocationContext, LocationList, LocationRecord, location_context, location_list, location_record,
};
use crate::mcp::arguments::{json, object, parse};
use crate::mcp::catalogue::Access;
use crate::mcp::definition::{Definition, Effect};
use crate::mcp::outcome::ToolError;
use crate::mcp::untrusted::cleaned;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationTool {
    List,
    Import,
    Context,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {
    /// Start of the span (RFC 3339, such as 2026-11-01T00:00:00Z).
    from: String,
    /// End of the span (RFC 3339), after `from` and at most 31 days from it.
    to: String,
    /// The most samples, 1 to 1000; 1000 when left out.
    first: Option<i64>,
}

/// One sample, as K1 `LocationSampleInput`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SampleInput {
    /// The identity of the sample at its source: the same key again is not stored twice.
    key: String,
    /// The time of the fix (RFC 3339); leave it out when the source gave none.
    observed_at: Option<String>,
    /// Give latitude and longitude together.
    latitude: Option<f64>,
    longitude: Option<f64>,
    /// Metres; leave it out when the source gave none. Never put a guess in its place.
    accuracy_m: Option<f64>,
    altitude_m: Option<f64>,
    speed_mps: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportInput {
    /// At most 500 samples.
    samples: Vec<SampleInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextInput {
    /// The ID of the transfer event.
    event: String,
}

impl LocationTool {
    pub const ALL: [Self; 3] = [Self::List, Self::Import, Self::Context];

    #[must_use]
    pub fn definition(self) -> Definition {
        match self {
            Self::List => Definition::new::<ListInput>(
                "anima_locations_list",
                Access::Read,
                Effect::Reads,
                "Lists the location samples of the operator's phone observed between two \
                 times, oldest first. Each has its provider, the time of the fix \
                 (`observedAt`), the time the hub got it (`receivedAt`) and a position; a \
                 number is null when the provider gave none, so a null accuracy is unknown, \
                 not exact. The newest sample is not where the operator was at an older \
                 event: use anima_location_context for an event. The hub writes an audit \
                 entry for each read. Positions are private data of the operator.",
            )
            .returns(&[("samples", "array", "The samples.")]),
            Self::Import => Definition::new::<ImportInput>(
                "anima_location_import",
                Access::Write,
                Effect::Adds,
                "Stores location samples that the operator gave you, at most 500 in one \
                 call. A sample with a key that the hub has is not stored again, so a repeat \
                 is safe. Keep the time of the fix as the source gave it, and leave out a \
                 number that the source did not give.",
            )
            .returns(&[("added", "integer", "How many samples were new.")]),
            Self::Context => Definition::new::<ContextInput>(
                "anima_location_context",
                Access::Read,
                Effect::Reads,
                "Shows the context of one file transfer event: the source and destination \
                 device with the site and place that each had at the event (configured, not \
                 measured; null is unknown), who started it, and separately the evidence for \
                 where the operator was. `operator.basis` is AT_THE_TIME, NEAREST_INFERRED \
                 or UNKNOWN; `operator.placeId` is given only for a sample at the time that \
                 is not uncertain. Do not take the place of a device as the place of the \
                 operator, or the reverse. `context` is null when the event is no transfer.",
            )
            .returns(&[("context", "object", "The context, or null.")]),
        }
    }

    /// Runs the tool.
    ///
    /// # Errors
    /// [`ToolError`] for wrong arguments or the hub's refusal.
    pub async fn call(self, hub: &Hub, arguments: Option<JsonObject>) -> Result<Value, ToolError> {
        let result = match self {
            Self::List => {
                let ListInput { from, to, first } = parse(arguments)?;
                let variables = location_list::Variables { from, to, first };
                let answer = hub.run::<LocationList>(variables).await?;
                object([("samples", cleaned(json(&answer.location_samples)))])
            }
            Self::Import => {
                let ImportInput { samples } = parse(arguments)?;
                let samples = samples
                    .into_iter()
                    .map(|sample| location_record::LocationSampleInput {
                        key: sample.key,
                        observed_at: sample.observed_at,
                        latitude: sample.latitude,
                        longitude: sample.longitude,
                        accuracy_m: sample.accuracy_m,
                        altitude_m: sample.altitude_m,
                        speed_mps: sample.speed_mps,
                    })
                    .collect();
                let variables = location_record::Variables { samples };
                let answer = hub.run::<LocationRecord>(variables).await?;
                object([("added", json(&answer.record_location_samples))])
            }
            Self::Context => {
                let ContextInput { event } = parse(arguments)?;
                let variables = location_context::Variables { event };
                let answer = hub.run::<LocationContext>(variables).await?;
                object([("context", cleaned(json(&answer.transfer_context)))])
            }
        };
        Ok(Value::Object(result))
    }
}
