//! The canonical form, the hash of a link and the walk of the check (LOG-022, LOG-023,
//! LOG-027), without a database.

use std::net::IpAddr;

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{
    canonical::canonical,
    check::{FaultKind, Walk},
    link::{FIRST_PREVIOUS, Hash, Link, StoredEntry, hash, hex},
};

fn entry(at_day: u32, name: &str) -> StoredEntry {
    StoredEntry {
        id: Uuid::now_v7(),
        at: Utc.with_ymd_and_hms(2031, 6, at_day, 12, 30, 0).unwrap(),
        operator_id: Uuid::nil(),
        operator_name: "operator".to_owned(),
        action: "update".to_owned(),
        target_type: "device".to_owned(),
        target_id: "lab".to_owned(),
        client: "100.64.0.7".parse().unwrap(),
        change: json!({ "name": name }),
        actor_kind: Some("user".to_owned()),
        token_kind: Some("login".to_owned()),
        api_key_name: None,
    }
}

/// Links each entry after the one before it, from link 1.
fn chain(entries: &[StoredEntry]) -> Vec<Link> {
    let mut previous: Hash = FIRST_PREVIOUS;
    (1..)
        .zip(entries)
        .map(|(seq, entry)| {
            let link = Link::new(seq, previous, entry);
            previous = link.hash;
            link
        })
        .collect()
}

fn first_kept() -> NaiveDate {
    NaiveDate::from_ymd_opt(2031, 6, 10).unwrap()
}

/// The walk over `links`, each with its entry when `entries` has it.
fn walk(links: &[Link], entries: &[Option<StoredEntry>]) -> super::ChainCheck {
    let mut walk = Walk::new(first_kept());
    for (link, entry) in links.iter().zip(entries) {
        if !walk.step(link, entry.as_ref()) {
            break;
        }
    }
    walk.finish(None)
}

#[test]
fn log_022_numbers_and_strings_are_written_as_rfc_8785_writes_them() {
    let cases = [
        (json!(0.0), "0"),
        (json!(-0.0), "0"),
        (json!(1.5), "1.5"),
        (json!(2000.0), "2000"),
        (json!(1e21), "1e+21"),
        (json!(1e20), "100000000000000000000"),
        (json!(0.000_001), "0.000001"),
        (json!(1e-7), "1e-7"),
        (json!(-1.25e-9), "-1.25e-9"),
        (json!(333_333_333.333_333_3), "333333333.3333333"),
        (json!(u64::MAX), "18446744073709551615"),
        (json!(-42), "-42"),
        (json!("a\"b\\c\n\u{f}é€"), "\"a\\\"b\\\\c\\n\\u000fé€\""),
    ];
    for (value, text) in cases {
        assert_eq!(canonical(&value), text, "for {value}");
    }
}

#[test]
fn log_022_members_are_ordered_by_utf_16_code_units() {
    let value: Value = serde_json::from_str(
        r#"{"€":1,"\r":2,"דּ":3,"1":4,"😀":5,"\u0080":6,"ö":7,"b":[true,null,{"z":0,"a":1}]}"#,
    )
    .unwrap();
    assert_eq!(
        canonical(&value),
        "{\"\\r\":2,\"1\":4,\"b\":[true,null,{\"a\":1,\"z\":0}],\"\u{80}\":6,\"ö\":7,\"€\":1,\"😀\":5,\"דּ\":3}"
    );
}

#[test]
fn log_022_the_hash_of_a_link_is_the_sha_256_of_its_14_members_in_canonical_form() {
    // The expected hash was made by a second implementation (Python `json.dumps` with sorted
    // keys and no spaces, then `hashlib.sha256`), which is RFC 8785 for this object.
    let entry = StoredEntry {
        id: "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d".parse().unwrap(),
        at: Utc.with_ymd_and_hms(2031, 6, 30, 12, 30, 0).unwrap()
            + chrono::TimeDelta::microseconds(123_456),
        operator_id: "0192f5a0-7c3e-7a52-9d1b-000000000001".parse().unwrap(),
        operator_name: "operator".to_owned(),
        action: "update".to_owned(),
        target_type: "device".to_owned(),
        target_id: "0192f5a0-7c3e-7a52-9d1b-3f4e5a6b7c8d".to_owned(),
        client: "2001:db8::7".parse::<IpAddr>().unwrap(),
        change: json!({ "tags": ["a", "b"], "port": 22, "name": "lab-router" }),
        actor_kind: Some("user".to_owned()),
        token_kind: Some("login".to_owned()),
        api_key_name: None,
    };
    assert_eq!(
        hex(&hash(2, &[0xab; 32], &entry)),
        "87fe77ba33a9fe0c0425bb1777e897a2e15346b86ac9a673fd75afe16e22fed0"
    );
}

#[test]
fn log_023_the_first_link_follows_64_zeros() {
    let links = chain(&[entry(20, "a")]);
    assert_eq!(links[0].seq, 1);
    assert_eq!(hex(&links[0].previous), "0".repeat(64));
}

#[test]
fn log_027_a_chain_of_several_entries_is_whole() {
    let entries: Vec<_> = (0..5).map(|n| entry(20, &n.to_string())).collect();
    let links = chain(&entries);
    let check = walk(&links, &entries.into_iter().map(Some).collect::<Vec<_>>());
    assert_eq!(check.fault, None);
    assert_eq!(check.links, 5);
    assert_eq!(check.head, Some((5, links[4].hash)));
}

#[test]
fn log_027_a_changed_field_is_found_at_its_link() {
    let mut entries: Vec<_> = (0..4).map(|n| Some(entry(20, &n.to_string()))).collect();
    let links = chain(&entries.iter().flatten().cloned().collect::<Vec<_>>());
    entries[2].as_mut().unwrap().operator_name = "someone else".to_owned();
    let fault = walk(&links, &entries).fault.unwrap();
    assert_eq!((fault.kind, fault.seq), (FaultKind::Changed, Some(3)));
}

#[test]
fn log_027_entries_that_changed_places_are_found_at_the_first_of_them() {
    let mut entries: Vec<_> = (0..4).map(|n| Some(entry(20, &n.to_string()))).collect();
    let links = chain(&entries.iter().flatten().cloned().collect::<Vec<_>>());
    entries.swap(1, 2);
    let fault = walk(&links, &entries).fault.unwrap();
    assert_eq!((fault.kind, fault.seq), (FaultKind::Changed, Some(2)));
}

#[test]
fn log_027_a_removed_entry_is_missing_inside_the_retention_and_dropped_before_it() {
    let mut entries = vec![
        Some(entry(1, "old")),
        Some(entry(20, "a")),
        Some(entry(20, "b")),
    ];
    let links = chain(&entries.iter().flatten().cloned().collect::<Vec<_>>());
    entries[0] = None;
    let check = walk(&links, &entries);
    assert_eq!((check.fault, check.dropped, check.links), (None, 1, 3));
    entries[2] = None;
    let fault = walk(&links, &entries).fault.unwrap();
    assert_eq!((fault.kind, fault.seq), (FaultKind::Missing, Some(3)));
}

#[test]
fn log_027_a_removed_or_moved_link_breaks_the_chain() {
    let entries: Vec<_> = (0..4).map(|n| Some(entry(20, &n.to_string()))).collect();
    let links = chain(&entries.iter().flatten().cloned().collect::<Vec<_>>());
    let gap = [links[0].clone(), links[2].clone()];
    let fault = walk(&gap, &[entries[0].clone(), entries[2].clone()])
        .fault
        .unwrap();
    assert_eq!((fault.kind, fault.seq), (FaultKind::Broken, Some(3)));
    let mut moved = links.clone();
    moved[1].previous = links[2].hash;
    let fault = walk(&moved, &entries).fault.unwrap();
    assert_eq!((fault.kind, fault.seq), (FaultKind::Broken, Some(2)));
}

#[test]
fn log_027_an_entry_without_a_link_is_reported_when_the_links_are_whole() {
    let stray = entry(20, "stray");
    let check = Walk::new(first_kept()).finish(Some((stray.id, stray.at)));
    let fault = check.fault.unwrap();
    assert_eq!(
        (fault.kind, fault.seq, fault.entry_id),
        (FaultKind::NotLinked, None, stray.id)
    );
}
