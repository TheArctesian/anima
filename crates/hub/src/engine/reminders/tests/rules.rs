//! The triggers that read occurrences, `unless`, the cooldown and the daily limit.

use std::collections::HashSet;

use chrono_tz::Tz;

use super::*;

#[test]
fn rem_001_a_source_event_fires_one_time_for_each_record_with_its_title_and_class() {
    let rule = rule(workout());
    let new = [
        record("strava", "workout", "1", "Morning run"),
        record("strava", "workout", "2", "Swim"),
        record("garmin", "workout", "3", "Evening run"),
    ];
    let known = Known::default();
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-16T12:00:00Z", &new, &known),
    );
    assert_eq!(due.len(), 1, "{due:?}");
    assert_eq!(
        (due[0].subject.as_str(), due[0].text.as_str(), due[0].class),
        ("workout:1", "Time to train: Morning run", Class::B)
    );
    assert_eq!((due[0].revision, due[0].stopped), (4, None));

    let history = History {
        subjects: HashSet::from(["workout:1".to_owned()]),
        ..History::default()
    };
    let again = evaluate(
        &rule,
        &history,
        &moment("2026-11-16T12:00:00Z", &new, &known),
    );
    assert_eq!(again, [], "one rule fires one time for one subject");
}

#[test]
fn rem_001_the_same_input_gives_the_same_output() {
    let rule = rule(workout());
    let new = [record("strava", "workout", "1", "Morning run")];
    let known = Known::default();
    let moment = moment("2026-11-16T12:00:00Z", &new, &known);
    assert_eq!(
        evaluate(&rule, &History::default(), &moment),
        evaluate(&rule, &History::default(), &moment)
    );
}

#[test]
fn rem_003_unless_a_record_of_today_is_known_or_new_the_rule_gives_nothing() {
    let reminder = Rule {
        unless: Some(Unless::RecordToday {
            source: "strava".to_owned(),
            kind: "workout".to_owned(),
        }),
        ..rule(daily((18, 0), Vec::new(), Tz::UTC))
    };
    let none = Known::default();
    let due = evaluate(
        &reminder,
        &History::default(),
        &moment("2026-11-16T18:00:30Z", &[], &none),
    );
    assert_eq!(due.len(), 1, "no workout today: the reminder is due");

    let known = Known {
        records_today: HashSet::from([("strava".to_owned(), "workout".to_owned())]),
        ..Known::default()
    };
    let quiet = evaluate(
        &reminder,
        &History::default(),
        &moment("2026-11-16T18:00:30Z", &[], &known),
    );
    assert_eq!(quiet, []);
    let new = [record("strava", "workout", "9", "Run")];
    let quiet = evaluate(
        &reminder,
        &History::default(),
        &moment("2026-11-16T18:00:30Z", &new, &none),
    );
    assert_eq!(quiet, [], "a workout that arrives now also stops it");
}

#[test]
fn rem_004_the_cooldown_and_the_daily_limit_stop_a_firing_and_say_why() {
    let rule = rule(workout());
    let new = [
        record("strava", "workout", "1", "Run 1"),
        record("strava", "workout", "2", "Run 2"),
    ];
    let known = Known::default();
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-16T12:00:00Z", &new, &known),
    );
    assert_eq!(
        [due[0].stopped, due[1].stopped],
        [None, Some(Stop::Cooldown)],
        "the second firing is inside the cooldown of the first"
    );

    let after_cooldown = History {
        last_fired: Some(at("2026-11-16T10:59:00Z")),
        fired_today: 3,
        subjects: HashSet::new(),
    };
    let due = evaluate(
        &rule,
        &after_cooldown,
        &moment("2026-11-16T12:00:00Z", &new[..1], &known),
    );
    assert_eq!(due[0].stopped, Some(Stop::DailyLimit));
    let inside = History {
        last_fired: Some(at("2026-11-16T11:00:01Z")),
        ..History::default()
    };
    let due = evaluate(
        &rule,
        &inside,
        &moment("2026-11-16T12:00:00Z", &new[..1], &known),
    );
    assert_eq!(
        due[0].stopped,
        Some(Stop::Cooldown),
        "59 minutes 59 seconds"
    );
}

#[test]
fn rem_001_a_threshold_fires_at_the_edge_and_not_while_the_measure_stays_there() {
    let rule = rule(Trigger::Threshold {
        key: "health.sleep_hours".to_owned(),
        is: Comparison::Below,
        value: 6.0,
    });
    let known = Known {
        measures: [("health.sleep_hours".to_owned(), 7.5)].into(),
        ..Known::default()
    };
    let new = [
        measure("health.sleep_hours", 5.0, "2026-11-16T07:00:00Z"),
        measure("health.sleep_hours", 4.0, "2026-11-17T07:00:00Z"),
        measure("money.spend_week", 1.0, "2026-11-17T07:00:00Z"),
    ];
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-17T08:00:00Z", &new, &known),
    );
    assert_eq!(due.len(), 1, "{due:?}");
    let still = Known {
        measures: [("health.sleep_hours".to_owned(), 5.0)].into(),
        ..Known::default()
    };
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-17T08:00:00Z", &new[1..], &still),
    );
    assert_eq!(due, [], "the measure before it was below too");
}

#[test]
fn rem_001_an_expiry_fires_one_time_for_each_number_of_days_and_each_record() {
    let rule = rule(Trigger::Expiry {
        subject: "documents.passport".to_owned(),
        days_before: vec![21, 14, 7],
    });
    let passport = Occurrence::Expiry {
        subject: "documents.passport".to_owned(),
        record: Reference::parse("document:p1").unwrap(),
        title: "Passport".to_owned(),
        class: Class::A,
        expires_at: at("2026-12-01T00:00:00Z"),
    };
    let known = Known::default();
    let new = [passport];
    let far = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-01T00:00:00Z", &new, &known),
    );
    assert_eq!(far, [], "30 days away");
    // First seen 10 days before: only the nearest number that applies fires, not 21 and 14.
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-21T00:00:00Z", &new, &known),
    );
    assert_eq!(due.len(), 1);
    assert_eq!(
        (due[0].subject.as_str(), due[0].class),
        ("document:p1:14", Class::A)
    );
    let history = History {
        subjects: HashSet::from(["document:p1:14".to_owned()]),
        ..History::default()
    };
    let same = evaluate(
        &rule,
        &history,
        &moment("2026-11-22T00:00:00Z", &new, &known),
    );
    assert_eq!(same, []);
    let week = evaluate(
        &rule,
        &history,
        &moment("2026-11-24T00:00:00Z", &new, &known),
    );
    assert_eq!(week[0].subject, "document:p1:7");
}

#[test]
fn rem_001_a_source_state_rule_reads_the_change_that_the_source_framework_gives() {
    let account = Reference::parse("source_account:a1").unwrap();
    let rule = rule(Trigger::SourceState {
        account: Some(account),
        state: "needs_reconnect".to_owned(),
    });
    let change = |id: &str, state: &str| Occurrence::Record {
        source: "sources".to_owned(),
        kind: state.to_owned(),
        record: Reference::parse(&format!("source_account:{id}")).unwrap(),
        title: "GitHub".to_owned(),
        class: Class::C,
        at: at("2026-11-16T12:00:00Z"),
    };
    let new = [
        change("a1", "stalled"),
        change("a2", "needs_reconnect"),
        change("a1", "needs_reconnect"),
    ];
    let known = Known::default();
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-16T12:00:00Z", &new, &known),
    );
    assert_eq!(due.len(), 1, "{due:?}");
    assert!(
        due[0]
            .subject
            .starts_with("source_account:a1:needs_reconnect:")
    );
}
