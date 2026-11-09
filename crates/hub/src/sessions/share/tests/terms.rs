//! The bounds of a share (D-9, S-3): `expiresInMinutes` 5..1440 and `maxViewers` 1..10, each
//! field outside them named once, in input order.

use chrono::TimeDelta;
use proptest::prelude::*;

use crate::sessions::share::{
    model::ShareAccess,
    terms::{Terms, checked_terms},
};

fn named(minutes: i32, viewers: i32) -> Vec<&'static str> {
    match checked_terms(ShareAccess::View, minutes, viewers) {
        Ok(_) => Vec::new(),
        Err(fields) => fields.iter().map(|field| field.field).collect(),
    }
}

proptest! {
    /// Accepted exactly within both bounds, with the lifetime and viewers asked for.
    #[test]
    fn d_9_terms_are_accepted_only_within_bounds(
        minutes in -10_i32..1500,
        viewers in -10_i32..20,
        control in any::<bool>(),
    ) {
        let access = if control { ShareAccess::Control } else { ShareAccess::View };
        let minutes_ok = (5..=1440).contains(&minutes);
        let viewers_ok = (1..=10).contains(&viewers);
        match checked_terms(access, minutes, viewers) {
            Ok(terms) => {
                prop_assert!(minutes_ok && viewers_ok);
                let expected = Terms {
                    access,
                    lifetime: TimeDelta::minutes(i64::from(minutes)),
                    max_viewers: u8::try_from(viewers).unwrap(),
                };
                prop_assert_eq!(terms, expected);
            }
            Err(fields) => {
                let expected: Vec<&str> = [
                    (!minutes_ok).then_some("expiresInMinutes"),
                    (!viewers_ok).then_some("maxViewers"),
                ]
                .into_iter()
                .flatten()
                .collect();
                prop_assert!(!expected.is_empty());
                let named: Vec<&str> = fields.iter().map(|field| field.field).collect();
                prop_assert_eq!(named, expected);
            }
        }
    }
}

#[test]
fn d_9_the_bounds_themselves_are_inside_and_their_neighbours_outside() {
    for (minutes, viewers) in [(5, 1), (1440, 10), (60, 3)] {
        assert_eq!(
            named(minutes, viewers),
            Vec::<&str>::new(),
            "{minutes} {viewers}"
        );
    }
    assert_eq!(named(4, 3), ["expiresInMinutes"]);
    assert_eq!(named(1441, 3), ["expiresInMinutes"]);
    assert_eq!(named(60, 0), ["maxViewers"]);
    assert_eq!(named(60, 11), ["maxViewers"]);
    assert_eq!(
        named(i32::MIN, i32::MAX),
        ["expiresInMinutes", "maxViewers"]
    );
}

#[test]
fn d_9_a_refused_field_says_its_bounds() {
    let Err(fields) = checked_terms(ShareAccess::View, 0, 0) else {
        panic!("both fields are out of bounds");
    };
    let messages: Vec<&str> = fields.iter().map(|field| field.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "Enter a value from 5 to 1440.",
            "Enter a value from 1 to 10."
        ]
    );
}
