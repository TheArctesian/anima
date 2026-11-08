//! Times in K8 bodies: RFC 3339 UTC with `Z` and whole seconds (K8 Section 1).

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize as _, Deserializer, Serializer};

/// Writes `at` as `2026-11-08T10:00:10Z`; a fraction of a second is cut off.
pub(super) fn serialize<S: Serializer>(at: &DateTime<Utc>, out: S) -> Result<S::Ok, S::Error> {
    out.serialize_str(&at.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// Reads an RFC 3339 time and holds it in UTC. Whether it is a whole multiple of 10 seconds is
/// the batch check's question.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<DateTime<Utc>, D::Error> {
    DateTime::<Utc>::deserialize(input)
}
