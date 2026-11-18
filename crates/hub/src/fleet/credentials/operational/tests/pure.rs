//! The due state, the read state of a version, and what the operator may enter (K-E5 Section
//! 2). Pure: the time is a parameter.

use chrono::{DateTime, TimeDelta, Utc};

use super::{Metadata, metadata};
use crate::fleet::credentials::operational::{
    Due, VersionState,
    lifecycle::{due, forget_before, read_state},
    rules::{is_name, is_workload, problems, value_problem},
};

fn at(day: i64) -> DateTime<Utc> {
    DateTime::UNIX_EPOCH + TimeDelta::days(day)
}

#[test]
fn r_147_the_due_state_is_expired_then_expiring_then_due() {
    let rotated = at(0);
    let every_90 = Metadata {
        rotation_days: Some(90),
        expires_at: None,
        ..metadata()
    };
    assert_eq!(due(&every_90, rotated, at(90)), Due::Current);
    assert_eq!(due(&every_90, rotated, at(91)), Due::Due);
    let expiring = Metadata {
        expires_at: Some(at(100)),
        ..every_90
    };
    assert_eq!(due(&expiring, rotated, at(85)), Due::Current);
    assert_eq!(
        due(&expiring, rotated, at(86)),
        Due::Expiring,
        "14 days before"
    );
    assert_eq!(
        due(&expiring, rotated, at(99)),
        Due::Expiring,
        "expiring wins over due"
    );
    assert_eq!(due(&expiring, rotated, at(100)), Due::Expired);
}

#[test]
fn r_147_only_an_active_version_of_an_expired_key_reads_as_expired() {
    let expiry = Some(at(10));
    assert_eq!(
        read_state(VersionState::Active, expiry, at(9)),
        VersionState::Active
    );
    assert_eq!(
        read_state(VersionState::Active, expiry, at(10)),
        VersionState::Expired
    );
    assert_eq!(
        read_state(VersionState::Revoked, expiry, at(11)),
        VersionState::Revoked
    );
    assert_eq!(
        read_state(VersionState::Active, None, at(11)),
        VersionState::Active
    );
}

#[test]
fn r_147_an_ended_value_is_kept_30_days() {
    assert_eq!(forget_before(at(40)), at(10));
}

#[test]
fn r_147_names_and_workloads_have_their_one_form() {
    for good in ["a", "cloudflare.dns", "x-1_y.z", &"a".repeat(80)] {
        assert!(is_name(good), "{good}");
    }
    for bad in ["", "1abc", "Cloudflare", "a b", "a/b", &"a".repeat(81)] {
        assert!(!is_name(bad), "{bad}");
    }
    assert!(is_workload(&"w".repeat(40)));
    assert!(!is_workload(&"w".repeat(41)));
}

#[test]
fn r_147_metadata_problems_name_their_fields() {
    assert_eq!(problems(&metadata()), Vec::new());
    let wrong = Metadata {
        provider: String::new(),
        purpose: "two\nlines".to_owned(),
        instructions: "x".repeat(4001),
        provider_link: Some("http://plain.example".to_owned()),
        rotation_days: Some(0),
        expires_at: None,
    };
    let fields: Vec<&str> = problems(&wrong).iter().map(|p| p.field).collect();
    assert_eq!(
        fields,
        [
            "provider",
            "purpose",
            "instructions",
            "providerLink",
            "rotationDays"
        ]
    );
    assert!(value_problem(0).is_some());
    assert!(value_problem(16 * 1024).is_none());
    assert!(value_problem(16 * 1024 + 1).is_some());
}
