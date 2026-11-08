//! The expiry date of an RDAP reply (D-29, RFC 9083): the expiration event, and replies that
//! give none or cannot be read.

use chrono::{DateTime, Utc};
use proptest::prelude::*;

use crate::checks::probe::rdap::reply::{Unreadable, expiration};

fn date(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn read(body: &str) -> Result<Option<DateTime<Utc>>, Unreadable> {
    expiration(body.as_bytes())
}

#[test]
fn d_29_the_expiration_event_gives_the_date() {
    let body = r#"{
      "objectClassName": "domain",
      "ldhName": "example.net",
      "events": [
        {"eventAction": "registration", "eventDate": "1995-08-14T04:00:00Z"},
        {"eventAction": "expiration", "eventDate": "2027-08-13T04:00:00Z"},
        {"eventAction": "last changed", "eventDate": "2026-08-14T07:01:44Z"}
      ]
    }"#;

    assert_eq!(read(body), Ok(Some(date("2027-08-13T04:00:00Z"))));
}

#[test]
fn d_29_offsets_and_fractions_are_read_as_utc() {
    let body = r#"{"events": [{"eventAction": "EXPIRATION", "eventDate": "2027-08-13T06:00:00.123+02:00"}]}"#;

    assert_eq!(read(body), Ok(Some(date("2027-08-13T04:00:00.123Z"))));
}

#[test]
fn d_29_with_several_expiration_events_the_earliest_counts() {
    let body = r#"{"events": [
        {"eventAction": "expiration", "eventDate": "2028-01-01T00:00:00Z"},
        {"eventAction": "expiration", "eventDate": "2027-01-01T00:00:00Z"},
        {"eventAction": "expiration", "eventDate": "2029-01-01T00:00:00Z"}
    ]}"#;

    assert_eq!(read(body), Ok(Some(date("2027-01-01T00:00:00Z"))));
}

#[test]
fn d_29_a_reply_without_a_readable_expiration_gives_no_date() {
    for body in [
        r#"{"objectClassName": "domain"}"#,
        r#"{"events": []}"#,
        r#"{"events": [{"eventAction": "registration", "eventDate": "1995-08-14T04:00:00Z"}]}"#,
        r#"{"events": [{"eventAction": "expiration"}]}"#,
        r#"{"events": [{"eventAction": "expiration", "eventDate": "next year"}]}"#,
        r#"{"events": [{"eventAction": "expiration", "eventDate": "2027-08-13"}]}"#,
        r#"{"events": [{"eventAction": "expiration", "eventDate": 1786593600}]}"#,
        r#"{"events": [{"eventAction": ["expiration"], "eventDate": "2027-08-13T04:00:00Z"}]}"#,
        r#"{"events": ["expiration", 7, null, []]}"#,
    ] {
        assert_eq!(read(body), Ok(None), "{body}");
    }
}

#[test]
fn d_29_a_malformed_date_beside_a_good_one_is_skipped() {
    let body = r#"{"events": [
        {"eventAction": "expiration", "eventDate": "soon"},
        {"eventAction": "expiration", "eventDate": "2027-08-13T04:00:00Z"}
    ]}"#;

    assert_eq!(read(body), Ok(Some(date("2027-08-13T04:00:00Z"))));
}

#[test]
fn d_29_a_reply_that_is_not_an_object_with_an_events_list_is_unreadable() {
    for body in [
        "",
        "null",
        "[]",
        "\"domain\"",
        "{\"events\": {}}",
        "{\"events\": [",
    ] {
        assert_eq!(read(body), Err(Unreadable), "{body}");
    }
}

proptest! {
    /// P3-003: the reply comes from a server the operator does not control, so no bytes make
    /// the parser panic.
    #[test]
    fn p3_003_any_bytes_parse_without_a_panic(body in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = expiration(&body);
    }

    /// P3-003: an event list of any actions and dates gives a date only from an expiration
    /// event with an RFC 3339 date.
    #[test]
    fn p3_003_any_events_give_only_an_expiration_date(
        events in proptest::collection::vec(("\\PC{0,12}", "\\PC{0,30}"), 0..8),
    ) {
        let list: Vec<_> = events
            .iter()
            .map(|(action, date)| serde_json::json!({ "eventAction": action, "eventDate": date }))
            .collect();
        let body = serde_json::json!({ "events": list }).to_string();
        let found = expiration(body.as_bytes()).unwrap();
        let expected = events
            .iter()
            .filter(|(action, _)| action.eq_ignore_ascii_case("expiration"))
            .filter_map(|(_, date)| DateTime::parse_from_rfc3339(date).ok())
            .map(|date| date.with_timezone(&Utc))
            .min();
        prop_assert_eq!(found, expected);
    }
}
