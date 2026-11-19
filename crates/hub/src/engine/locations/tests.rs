//! The pure rules of the locations domain: the match of an event with a sample (R-145), the
//! OwnTracks form and the check of a sample.

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;

use super::{
    Basis, NewSample, Position, Provider, RULE, Sample, model::faults, operator_evidence, owntracks,
};
use crate::{
    engine::places::{Category, Centre, Place},
    ids::{PlaceId, SampleId},
};

fn event_at() -> DateTime<Utc> {
    DateTime::from_timestamp(1_794_830_400, 0).unwrap()
}

/// A sample `seconds` from the event at the centre of Home, with `accuracy`.
fn sample(number: u128, seconds: i64, accuracy: Option<f64>) -> Sample {
    Sample {
        id: SampleId::from(Uuid::from_u128(number)),
        provider: Provider::OwnTracks,
        account: "phone".to_owned(),
        key: format!("p:{number}"),
        observed_at: Some(event_at() + TimeDelta::seconds(seconds)),
        received_at: event_at(),
        position: Position {
            latitude: Some(52.0),
            longitude: Some(4.0),
            accuracy_m: accuracy,
            ..Position::default()
        },
    }
}

fn place(number: u128, (latitude, longitude): (f64, f64), radius_m: u32) -> Place {
    Place {
        id: PlaceId::from(Uuid::from_u128(number)),
        parent: None,
        name: format!("Place {number}"),
        centre: Some(Centre {
            latitude,
            longitude,
            radius_m,
        }),
        category: Category::parse("home").unwrap(),
        icon: "house".to_owned(),
        aliases: Vec::new(),
        geocoder_name: None,
        merged_into: None,
        created_at: event_at(),
        updated_at: event_at(),
    }
}

fn home() -> Vec<Place> {
    vec![place(1, (52.0, 4.0), 100)]
}

#[test]
fn r_145_a_sample_within_two_minutes_is_evidence_at_the_time_with_its_place() {
    let evidence = operator_evidence(event_at(), &[sample(7, -120, Some(15.0))], &home(), &RULE);
    assert_eq!(evidence.basis, Basis::AtTheTime);
    assert_eq!(evidence.sample, Some(SampleId::from(Uuid::from_u128(7))));
    assert_eq!(evidence.offset_seconds, Some(-120));
    assert!(!evidence.uncertain);
    assert_eq!(evidence.place, Some(home()[0].id));
}

#[test]
fn r_145_the_nearest_sample_within_ten_minutes_is_inferred_and_is_no_confirmed_place() {
    let near = [sample(1, 121, Some(15.0)), sample(2, -500, Some(15.0))];
    let evidence = operator_evidence(event_at(), &near, &home(), &RULE);
    assert_eq!(evidence.basis, Basis::NearestInferred);
    assert_eq!(evidence.sample, Some(near[0].id));
    assert_eq!(evidence.place, None);
    let edge = operator_evidence(event_at(), &[sample(3, 600, Some(15.0))], &home(), &RULE);
    assert_eq!(edge.basis, Basis::NearestInferred);
}

#[test]
fn r_145_no_sample_within_ten_minutes_is_unknown_with_no_sample() {
    let far = [sample(1, 601, Some(15.0)), sample(2, -86_400, Some(15.0))];
    let evidence = operator_evidence(event_at(), &far, &home(), &RULE);
    assert_eq!(
        (evidence.basis, evidence.sample, evidence.place),
        (Basis::Unknown, None, None)
    );
    let timeless = Sample {
        observed_at: None,
        ..sample(3, 0, Some(15.0))
    };
    let evidence = operator_evidence(event_at(), &[timeless], &home(), &RULE);
    assert_eq!(evidence.basis, Basis::Unknown, "no time never matches");
}

#[test]
fn r_145_a_sample_with_no_accuracy_or_worse_than_200_m_is_uncertain_and_no_place() {
    for accuracy in [None, Some(200.1)] {
        let evidence = operator_evidence(event_at(), &[sample(1, 0, accuracy)], &home(), &RULE);
        assert_eq!(evidence.basis, Basis::AtTheTime);
        assert!(evidence.uncertain, "{accuracy:?}");
        assert_eq!(evidence.place, None, "{accuracy:?}");
    }
    let exact = operator_evidence(event_at(), &[sample(1, 0, Some(200.0))], &home(), &RULE);
    assert!(!exact.uncertain);
}

#[test]
fn r_145_of_two_samples_at_the_same_distance_the_earlier_one_decides() {
    let near = [sample(1, 30, Some(15.0)), sample(2, -30, Some(15.0))];
    let evidence = operator_evidence(event_at(), &near, &home(), &RULE);
    assert_eq!(evidence.sample, Some(near[1].id));
}

#[test]
fn r_145_a_late_or_newer_sample_does_not_change_the_evidence_of_an_older_event() {
    let at_the_time = sample(1, 10, Some(15.0));
    let before = operator_evidence(
        event_at(),
        std::slice::from_ref(&at_the_time),
        &home(),
        &RULE,
    );
    // A newer position at a different place, and the same sample received a day late.
    let newer = Sample {
        position: Position {
            latitude: Some(48.0),
            longitude: Some(11.0),
            accuracy_m: Some(5.0),
            ..Position::default()
        },
        ..sample(2, 86_400, Some(5.0))
    };
    let late = Sample {
        received_at: event_at() + TimeDelta::days(1),
        ..at_the_time
    };
    let after = operator_evidence(event_at(), &[newer, late], &home(), &RULE);
    assert_eq!(after, before);
}

#[test]
fn plc_002_the_place_is_the_top_level_one_whose_radius_contains_the_sample() {
    // About 111 m north of the sample: outside a radius of 100 m, inside one of 150 m.
    let outside = vec![place(1, (52.001, 4.0), 100)];
    let fix = [sample(1, 0, Some(15.0))];
    assert_eq!(
        operator_evidence(event_at(), &fix, &outside, &RULE).place,
        None
    );
    let both = vec![place(1, (52.001, 4.0), 150), place(2, (52.0003, 4.0), 150)];
    let evidence = operator_evidence(event_at(), &fix, &both, &RULE);
    assert_eq!(evidence.place, Some(both[1].id), "the nearest centre");
    let child = Place {
        parent: Some(both[0].id),
        ..place(3, (52.0, 4.0), 100)
    };
    assert_eq!(
        operator_evidence(event_at(), &fix, &[child], &RULE).place,
        None
    );
}

#[test]
fn r_145_an_owntracks_location_gives_its_time_its_key_and_its_numbers() {
    let body = br#"{"_type":"location","tst":1794830400,"lat":52.1,"lon":4.2,"acc":12,
                    "alt":3,"vel":36,"tid":"ph","batt":80}"#;
    let sample = owntracks::sample(body).unwrap().unwrap();
    assert_eq!(sample.key, "ph:1794830400");
    assert_eq!(sample.observed_at, Some(event_at()));
    assert_eq!(
        sample.position,
        Position {
            latitude: Some(52.1),
            longitude: Some(4.2),
            accuracy_m: Some(12.0),
            altitude_m: Some(3.0),
            speed_mps: Some(10.0),
        }
    );
}

#[test]
fn r_145_an_owntracks_location_with_no_accuracy_keeps_none() {
    let body = br#"{"_type":"location","tst":1794830400,"lat":52.1,"lon":4.2}"#;
    let sample = owntracks::sample(body).unwrap().unwrap();
    assert_eq!(sample.position.accuracy_m, None);
    assert_eq!(sample.position.speed_mps, None);
    assert_eq!(sample.key, ":1794830400");
}

#[test]
fn r_145_another_owntracks_type_is_accepted_and_gives_no_sample() {
    assert_eq!(
        owntracks::sample(br#"{"_type":"transition","tst":1}"#),
        Ok(None)
    );
    assert!(owntracks::sample(br#"{"_type":"location","lat":1.0,"lon":2.0}"#).is_err());
    assert!(owntracks::sample(b"[1, 2]").is_err());
}

#[test]
fn r_145_a_sample_names_each_field_that_is_not_valid() {
    let sample = NewSample {
        key: String::new(),
        observed_at: None,
        position: Position {
            latitude: Some(91.0),
            longitude: None,
            accuracy_m: Some(-1.0),
            altitude_m: None,
            speed_mps: Some(f64::NAN),
        },
    };
    let fields: Vec<String> = faults(2, &sample)
        .into_iter()
        .map(|bad| bad.field)
        .collect();
    assert_eq!(
        fields,
        [
            "samples[2].key",
            "samples[2].latitude",
            "samples[2].latitude",
            "samples[2].accuracyM",
            "samples[2].speedMps"
        ]
    );
    let bare = NewSample {
        key: "a".to_owned(),
        observed_at: None,
        position: Position::default(),
    };
    assert_eq!(faults(0, &bare), []);
}
