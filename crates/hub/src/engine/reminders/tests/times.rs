//! The instants of a time rule.

use std::collections::HashSet;

use chrono::Weekday;
use chrono_tz::Tz;

use super::*;

#[test]
fn rem_002_a_time_rule_that_was_missed_fires_one_time_for_its_latest_instant() {
    let rule = rule(daily((9, 0), Vec::new(), Tz::UTC));
    let known = Known::default();
    // The hub was stopped for three days.
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-19T15:00:00Z", &[], &known),
    );
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].subject, "time:2026-11-19T09:00:00Z");
    let before = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-19T08:59:59Z", &[], &known),
    );
    assert_eq!(before[0].subject, "time:2026-11-18T09:00:00Z");
}

#[test]
fn rem_005_a_time_before_the_last_change_of_the_rule_is_not_due() {
    let changed = Rule {
        updated_at: at("2026-11-19T10:00:00Z"),
        ..rule(daily((9, 0), Vec::new(), Tz::UTC))
    };
    let known = Known::default();
    let due = evaluate(
        &changed,
        &History::default(),
        &moment("2026-11-19T15:00:00Z", &[], &known),
    );
    assert_eq!(due, [], "the rule did not exist in this form at 09:00");
    let once = Rule {
        trigger: Trigger::Time(TimeTrigger::Once(at("2026-11-19T12:00:00Z"))),
        ..changed
    };
    let due = evaluate(
        &once,
        &History::default(),
        &moment("2026-11-19T12:00:10Z", &[], &known),
    );
    assert_eq!(due[0].subject, "time:2026-11-19T12:00:00Z");
    let early = evaluate(
        &once,
        &History::default(),
        &moment("2026-11-19T11:59:59Z", &[], &known),
    );
    assert_eq!(early, []);
}

#[test]
fn rem_002_a_time_rule_keeps_its_days_and_the_local_time_of_its_zone() {
    let zone: Tz = "Europe/Amsterdam".parse().unwrap();
    let rule = rule(daily((9, 0), vec![Weekday::Mon], zone));
    let known = Known::default();
    // 2026-11-16 is a Monday; 09:00 in Amsterdam is 08:00 UTC in winter.
    let due = evaluate(
        &rule,
        &History::default(),
        &moment("2026-11-18T12:00:00Z", &[], &known),
    );
    assert_eq!(due[0].subject, "time:2026-11-16T08:00:00Z");
    assert_eq!(
        day_start(&rule, at("2026-11-16T12:00:00Z")),
        at("2026-11-15T23:00:00Z"),
        "the day of the limit is the local day"
    );
}

#[test]
fn rem_002_a_skipped_local_time_is_due_at_the_next_minute_and_a_repeated_one_fires_once() {
    let zone: Tz = "Europe/Amsterdam".parse().unwrap();
    let known = Known::default();
    // 2026-03-29: 02:00 to 03:00 does not exist in Amsterdam.
    let skipped = rule(daily((2, 30), Vec::new(), zone));
    let due = evaluate(
        &skipped,
        &History::default(),
        &moment("2026-03-29T06:00:00Z", &[], &known),
    );
    assert_eq!(due[0].subject, "time:2026-03-29T01:00:00Z", "03:00 local");
    // 2026-10-25: 02:30 happens two times; the earlier one (00:30 UTC) is the one.
    let repeated = rule(daily((2, 30), Vec::new(), zone));
    let first = evaluate(
        &repeated,
        &History::default(),
        &moment("2026-10-25T00:45:00Z", &[], &known),
    );
    assert_eq!(first[0].subject, "time:2026-10-25T00:30:00Z");
    let history = History {
        subjects: HashSet::from([first[0].subject.clone()]),
        ..History::default()
    };
    let second = evaluate(
        &repeated,
        &history,
        &moment("2026-10-25T01:45:00Z", &[], &known),
    );
    assert_eq!(second, []);
}
