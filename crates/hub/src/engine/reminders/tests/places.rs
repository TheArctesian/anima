//! The place state and the place rule (REM-006).

use chrono::TimeDelta;
use uuid::Uuid;

use super::*;

fn home() -> Place {
    Place {
        id: PlaceId::from(Uuid::from_u128(7)),
        parent: None,
        name: "Home".to_owned(),
        centre: Some(Centre {
            latitude: 52.0,
            longitude: 4.0,
            radius_m: 100,
        }),
        category: Category::parse("home").unwrap(),
        icon: "house".to_owned(),
        aliases: Vec::new(),
        geocoder_name: None,
        merged_into: None,
        created_at: at("2026-01-01T00:00:00Z"),
        updated_at: at("2026-01-01T00:00:00Z"),
    }
}

/// A fix `minutes` after noon, at home or 1 km away, with `accuracy`.
fn fix(number: u128, minutes: i64, at_home: bool, accuracy: Option<f64>) -> Sample {
    Sample {
        id: SampleId::from(Uuid::from_u128(number)),
        provider: Provider::OwnTracks,
        account: "phone".to_owned(),
        key: number.to_string(),
        observed_at: Some(at("2026-11-16T12:00:00Z") + TimeDelta::minutes(minutes)),
        received_at: at("2026-11-16T13:00:00Z"),
        position: Position {
            latitude: Some(if at_home { 52.0 } else { 52.01 }),
            longitude: Some(4.0),
            accuracy_m: accuracy,
            ..Position::default()
        },
    }
}

#[test]
fn rem_006_a_fix_inside_is_an_arrival_only_when_the_last_state_was_outside() {
    let places = [home()];
    let samples = [
        fix(3, 2, true, Some(10.0)),
        fix(1, 0, false, Some(10.0)),
        fix(2, 1, true, Some(10.0)),
        fix(4, 3, false, Some(10.0)),
    ];
    let changes = place_changes(&[], &samples, &places);
    let seen: Vec<(u128, bool)> = changes
        .iter()
        .map(|change| (Uuid::from(change.sample).as_u128(), change.inside))
        .collect();
    assert_eq!(
        seen,
        [(2, true), (4, false)],
        "in the order of the fixes; no state is outside"
    );
}

#[test]
fn rem_006_an_uncertain_or_an_older_fix_changes_no_state() {
    let places = [home()];
    let uncertain = [fix(1, 0, true, None), fix(2, 1, true, Some(200.5))];
    assert_eq!(place_changes(&[], &uncertain, &places), []);
    let inside = [PlaceState {
        place: places[0].id,
        inside: true,
        changed_at: at("2026-11-16T12:30:00Z"),
        sample: SampleId::from(Uuid::from_u128(9)),
    }];
    // A late import of a fix outside, from before the arrival.
    let late = [fix(3, 10, false, Some(10.0))];
    assert_eq!(place_changes(&inside, &late, &places), []);
    let left = [fix(4, 45, false, Some(10.0))];
    assert_eq!(place_changes(&inside, &left, &places).len(), 1);
}

#[test]
fn rem_006_a_place_rule_fires_on_its_crossing_and_only_for_a_fresh_fix() {
    let places = [home()];
    let arrive = rule(Trigger::Place {
        place: places[0].id,
        on: Crossing::Arrive,
    });
    let leave = Rule {
        trigger: Trigger::Place {
            place: places[0].id,
            on: Crossing::Leave,
        },
        ..arrive.clone()
    };
    let changes = place_changes(&[], &[fix(1, 0, true, Some(10.0))], &places);
    let known = Known::default();
    let fresh = Moment {
        now: at("2026-11-16T12:09:00Z"),
        new: &[],
        places: &changes,
        known: &known,
    };
    let due = evaluate(&arrive, &History::default(), &fresh);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].class, Class::B, "where the operator is, is Class B");
    assert_eq!(evaluate(&leave, &History::default(), &fresh), []);
    let stale = Moment {
        now: at("2026-11-16T12:10:01Z"),
        ..fresh
    };
    assert_eq!(
        evaluate(&arrive, &History::default(), &stale),
        [],
        "older than 10 minutes"
    );
}
