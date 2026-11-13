//! The expiry date of an RDAP domain reply (RFC 9083 Section 4.5): the `eventDate` of its
//! `expiration` event. Pure: the reply comes in as bytes, and nothing in it is trusted.

use chrono::{DateTime, Utc};
use serde_json::Value;

/// The reply is not a JSON object, or its `events` member is not a list.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("the RDAP reply cannot be read")]
pub(super) struct Unreadable;

/// The registration expiry the reply gives, or `None` when it gives none: no `events`, no
/// `expiration` event, or none whose date is an RFC 3339 instant. With several, the earliest,
/// so the warning comes no later than any date the registry states. `eventAction` is compared
/// without regard to ASCII case.
///
/// # Errors
/// [`Unreadable`] when the body is not a JSON object or `events` is not a list.
pub(super) fn expiration(body: &[u8]) -> Result<Option<DateTime<Utc>>, Unreadable> {
    let reply: Value = serde_json::from_slice(body).map_err(|_| Unreadable)?;
    let object = reply.as_object().ok_or(Unreadable)?;
    let Some(events) = object.get("events") else {
        return Ok(None);
    };
    let events = events.as_array().ok_or(Unreadable)?;
    Ok(events.iter().filter_map(expiration_date).min())
}

/// The date of one event when it is an `expiration` event with a readable date.
fn expiration_date(event: &Value) -> Option<DateTime<Utc>> {
    let action = event.get("eventAction")?.as_str()?;
    if !action.eq_ignore_ascii_case("expiration") {
        return None;
    }
    let date = event.get("eventDate")?.as_str()?;
    DateTime::parse_from_rfc3339(date)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}
