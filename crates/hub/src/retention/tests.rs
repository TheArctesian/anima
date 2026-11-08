//! Unit tests of the pure rules. Behaviour against the database and GraphQL is tested in
//! `tests/it/retention`.

use chrono::Days;

use super::{
    Error, Periods, Subject,
    bounds::{Bounds, bounds, checked},
};

fn rows(days: [i32; 5]) -> Vec<(String, i32)> {
    Subject::ALL
        .into_iter()
        .zip(days)
        .map(|(subject, days)| (subject.as_str().to_owned(), days))
        .collect()
}

#[test]
fn hub_005_each_subject_has_the_range_and_default_of_d_25() {
    let expected = [
        (Subject::Heartbeats, (1, 400, 30)),
        (Subject::Metrics, (1, 400, 30)),
        (Subject::Events, (30, 365, 90)),
        (Subject::Recordings, (1, 365, 30)),
        (Subject::Deliveries, (7, 365, 30)),
    ];
    for (subject, (min, max, default)) in expected {
        assert_eq!(bounds(subject), Bounds { min, max, default }, "{subject:?}");
    }
}

#[test]
fn hub_005_a_period_one_inside_and_one_outside_each_end_is_accepted_or_refused() {
    for subject in Subject::ALL {
        let Bounds { min, max, .. } = bounds(subject);
        let (min, max) = (i64::from(min), i64::from(max));
        assert!(checked(subject, min - 1).is_err(), "{subject:?} below");
        assert_eq!(checked(subject, min).map(i64::from), Ok(min), "{subject:?}");
        assert_eq!(checked(subject, max).map(i64::from), Ok(max), "{subject:?}");
        assert!(checked(subject, max + 1).is_err(), "{subject:?} above");
    }
}

#[test]
fn hub_005_a_refused_period_names_the_days_field_and_not_the_value() {
    let refused = checked(Subject::Events, -7).unwrap_err();
    assert_eq!(refused.field, "days");
    assert!(!refused.message.contains("-7"), "{}", refused.message);
}

#[test]
fn hub_005_periods_turn_the_stored_days_into_a_keep_period_for_each_subject() {
    let periods = Periods::from_rows(&rows([5, 6, 40, 8, 9])).unwrap();
    let keep: Vec<_> = Subject::ALL.into_iter().map(|s| periods.keep(s)).collect();
    let days = [5, 6, 40, 8, 9].map(Days::new);
    assert_eq!(keep, days);
}

#[test]
fn hub_005_a_missing_or_out_of_bounds_stored_period_is_unreadable_not_a_default() {
    let mut missing = rows([30, 30, 90, 30, 30]);
    missing.pop();
    let outside = rows([30, 30, 10, 30, 30]);
    for stored in [missing, outside] {
        assert!(matches!(
            Periods::from_rows(&stored),
            Err(Error::Unreadable)
        ));
    }
}
