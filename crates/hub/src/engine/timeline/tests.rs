//! The timeline's rules over evidence from made-up adapters: nothing is copied or dropped,
//! and two sources of one event are both kept (TML-001, TML-003 to TML-006).

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};

use super::{Ask, Basis, Category, Evidence, Mark, arrange};
use crate::engine::records::Reference;

fn hour(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 1, 1, hour, 0, 0).unwrap()
}

fn entry(reference: &str, source: (&str, u16), start: DateTime<Utc>) -> Evidence {
    Evidence {
        reference: Reference::parse(reference).unwrap(),
        source: source.0.to_owned(),
        rank: source.1,
        category: Category::Workout,
        title: reference.to_owned(),
        start,
        end: None,
        received_at: None,
        basis: Basis::Observed,
        uncertainty: Vec::new(),
        occurrence: None,
        superseded_by: None,
    }
}

fn same_event(mut entry: Evidence, occurrence: &str) -> Evidence {
    entry.occurrence = Some(occurrence.to_owned());
    entry
}

fn day(limit: usize) -> Ask {
    Ask {
        from: hour(0),
        to: hour(23),
        after: None,
        categories: None,
        limit,
    }
}

fn references(entries: &[Evidence]) -> Vec<&str> {
    entries.iter().map(|e| e.reference.as_str()).collect()
}

#[test]
fn tml_005_two_sources_of_one_event_are_both_kept_and_the_lower_points_to_the_higher() {
    let watch = same_event(
        entry("garmin_activity:7", ("garmin", 80), hour(7)),
        "run-0700",
    );
    let phone = same_event(
        entry("apple_workout:3", ("apple_health", 40), hour(7)),
        "run-0700",
    );
    let alone = entry("apple_workout:4", ("apple_health", 40), hour(9));
    for order in [
        vec![watch.clone(), phone.clone(), alone.clone()],
        vec![alone, phone, watch],
    ] {
        let page = arrange(order, &day(10));
        assert_eq!(
            references(&page.entries),
            ["apple_workout:4", "garmin_activity:7", "apple_workout:3"]
        );
        let leaders: Vec<_> = (page.entries.iter())
            .map(|e| e.superseded_by.as_ref().map(Reference::as_str))
            .collect();
        assert_eq!(leaders, [None, None, Some("garmin_activity:7")]);
    }
}

#[test]
fn tml_005_only_the_same_occurrence_in_the_same_category_from_another_source_is_superseded() {
    let leader = same_event(entry("garmin_activity:7", ("garmin", 80), hour(7)), "a");
    let same_source = same_event(entry("garmin_activity:8", ("garmin", 80), hour(7)), "a");
    let other_occurrence = same_event(entry("apple_workout:1", ("apple_health", 40), hour(7)), "b");
    let mut other_category = same_event(entry("sleep:1", ("apple_health", 40), hour(7)), "a");
    other_category.category = Category::Sleep;
    let mut already = same_event(entry("strava:1", ("strava", 10), hour(7)), "a");
    already.superseded_by = Some(Reference::parse("apple_workout:1").unwrap());
    let page = arrange(
        vec![
            leader,
            same_source,
            other_occurrence,
            other_category,
            already,
        ],
        &day(10),
    );
    assert_eq!(page.entries.len(), 5, "no entry is removed");
    let pointed: Vec<_> = (page.entries.iter())
        .filter_map(|e| Some((e.reference.as_str(), e.superseded_by.as_ref()?.as_str())))
        .collect();
    assert_eq!(pointed, [("strava:1", "apple_workout:1")]);
}

#[test]
fn tml_005_equal_ranks_still_give_one_leader() {
    let a = same_event(entry("a_run:1", ("alpha", 50), hour(7)), "run");
    let b = same_event(entry("b_run:1", ("beta", 50), hour(7)), "run");
    for order in [vec![a.clone(), b.clone()], vec![b, a]] {
        let page = arrange(order, &day(10));
        let pointed: Vec<_> = (page.entries.iter())
            .map(|e| e.superseded_by.as_ref().map(Reference::as_str))
            .collect();
        assert_eq!(references(&page.entries), ["b_run:1", "a_run:1"]);
        assert_eq!(pointed, [Some("a_run:1"), None]);
    }
}

#[test]
fn tml_001_a_page_holds_only_what_was_asked_newest_first_and_each_record_once() {
    let early = entry("note:1", ("notes", 1), hour(0) - TimeDelta::seconds(1));
    let at_end = entry("note:2", ("notes", 1), hour(23));
    let mut sleep = entry("sleep:1", ("notes", 1), hour(3));
    sleep.category = Category::Sleep;
    let kept = [
        entry("note:3", ("notes", 1), hour(5)),
        entry("note:4", ("notes", 1), hour(5)),
        entry("note:5", ("notes", 1), hour(0)),
    ];
    let mut all = vec![early, at_end, sleep, kept[0].clone()];
    all.extend(kept.clone());
    let ask = Ask {
        categories: Some(vec![Category::Workout]),
        ..day(2)
    };
    let first = arrange(all.clone(), &ask);
    assert_eq!(references(&first.entries), ["note:4", "note:3"]);
    assert!(first.has_next_page);
    let rest = Ask {
        after: Some(Mark {
            start: hour(5),
            reference: Reference::parse("note:3").unwrap(),
        }),
        ..ask
    };
    let second = arrange(all, &rest);
    assert_eq!(references(&second.entries), ["note:5"]);
    assert!(!second.has_next_page);
}

#[test]
fn tml_012_a_replayed_record_is_one_entry_and_another_source_of_its_event_is_a_second() {
    let first = same_event(entry("garmin_activity:7", ("garmin", 80), hour(7)), "run");
    let replay = first.clone();
    let other = same_event(
        entry("apple_workout:3", ("apple_health", 40), hour(7)),
        "run",
    );
    let page = arrange(vec![first, replay, other], &day(10));
    assert_eq!(
        references(&page.entries),
        ["garmin_activity:7", "apple_workout:3"]
    );
    assert_eq!(
        page.entries[1]
            .superseded_by
            .as_ref()
            .map(Reference::as_str),
        Some("garmin_activity:7")
    );
}

#[test]
fn tml_009_a_record_received_late_keeps_its_event_time_and_sorts_by_it() {
    let mut imported = entry("garmin_activity:7", ("garmin", 80), hour(3));
    imported.received_at = Some(hour(20));
    let recent = entry("note:1", ("notes", 1), hour(9));
    let page = arrange(vec![imported, recent], &day(10));
    assert_eq!(references(&page.entries), ["note:1", "garmin_activity:7"]);
    assert_eq!(page.entries[1].start, hour(3));
    assert_eq!(page.entries[1].received_at, Some(hour(20)));
}
