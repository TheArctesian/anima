//! An API key's pace (K-E1 Section 13.1): 20 changes in the minute from its first, then none
//! until that minute ends; one key's minute holds no other key back.

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;

use crate::{ids::TokenId, work::projects::KeyChanges};

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000, 0).unwrap() + TimeDelta::seconds(seconds)
}

fn key(n: u128) -> TokenId {
    TokenId::from(Uuid::from_u128(n))
}

#[test]
fn od_e14_a_key_makes_20_changes_in_its_minute_and_none_more_until_it_ends() {
    let changes = KeyChanges::default();
    for n in 0..20 {
        assert!(changes.take(key(1), at(n * 2)), "change {n}");
    }
    assert!(!changes.take(key(1), at(40)), "the 21st in the minute");
    assert!(!changes.take(key(1), at(59)), "still the same minute");
    assert!(changes.take(key(1), at(60)), "a new minute");
}

#[test]
fn od_e14_one_keys_minute_holds_no_other_key_back() {
    let changes = KeyChanges::default();
    for _ in 0..20 {
        assert!(changes.take(key(1), at(0)));
    }
    assert!(!changes.take(key(1), at(1)));
    assert!(
        changes.take(key(2), at(1)),
        "another key has its own minute"
    );
}
