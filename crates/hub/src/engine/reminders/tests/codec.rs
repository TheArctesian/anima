//! The stored form of a trigger and of `unless` (K-E6 Section 2).

use serde_json::json;

use super::*;

#[test]
fn rem_001_each_trigger_reads_from_its_stored_form_and_writes_back_equal() {
    let stored = [
        json!({ "type": "time", "at": "07:30", "days": ["mon", "fri"], "zone": "Europe/Amsterdam" }),
        json!({ "type": "time", "once": "2026-12-01T09:00:00Z" }),
        json!({ "type": "place", "place": "0190a000-0000-7000-9200-000000000001", "on": "leave" }),
        json!({ "type": "source_event", "source": "strava", "kind": "workout", "contains": "run" }),
        json!({ "type": "source_state", "account": null, "state": "needs_reconnect" }),
        json!({ "type": "threshold", "key": "health.sleep_hours", "is": "below", "value": 6.0 }),
        json!({ "type": "expiry", "subject": "documents.passport", "days_before": [21, 14, 7] }),
    ];
    for json in stored {
        let read = trigger(&json).unwrap_or_else(|fault| panic!("{json}: {fault:?}"));
        assert_eq!(trigger(&trigger_json(&read)), Ok(read), "{json}");
    }
    let condition = json!({ "record_today": { "source": "strava", "kind": "workout" } });
    let read = unless(&condition).unwrap();
    assert_eq!(unless_json(&read), condition);
}

#[test]
fn rem_001_a_trigger_that_is_not_valid_names_the_field_to_fix() {
    let cases = [
        (
            json!({ "type": "time", "at": "25:00", "zone": "UTC" }),
            "trigger.at",
        ),
        (
            json!({ "type": "time", "at": "07:30", "zone": "Mars/Olympus" }),
            "trigger.zone",
        ),
        (
            json!({ "type": "time", "at": "07:30", "zone": "UTC", "days": ["someday"] }),
            "trigger.days",
        ),
        (
            json!({ "type": "time", "at": "07:30", "once": "2026-12-01T09:00:00Z" }),
            "trigger.once",
        ),
        (
            json!({ "type": "source_state", "state": "broken" }),
            "trigger.state",
        ),
        (
            json!({ "type": "threshold", "key": "sleep", "is": "below", "value": 6.0 }),
            "trigger.key",
        ),
        (
            json!({ "type": "expiry", "subject": "documents.passport", "days_before": [] }),
            "trigger.days_before",
        ),
        (json!({ "type": "weather" }), "trigger"),
        (
            json!({ "type": "place", "place": "home", "on": "arrive" }),
            "trigger",
        ),
    ];
    for (json, field) in cases {
        assert_eq!(
            trigger(&json).map_err(|fault| fault.field),
            Err(field),
            "{json}"
        );
    }
    let both = json!({ "record_today": { "source": "a", "kind": "b" }, "measure": {} });
    assert_eq!(unless(&both).map_err(|fault| fault.field), Err("unless"));
}
