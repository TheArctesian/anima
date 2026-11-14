//! SRC-007: a listing with no end does not grow the stored slice without limit.

use super::*;

fn full_page_without_a_total(first_id: u64) -> Vec<u8> {
    let entries: Vec<Value> = (first_id..first_id + 100)
        .map(|id| repository(id, &format!("arctesian/r{id}"), "2026-10-01T08:00:00Z"))
        .collect();
    serde_json::to_vec(&json!({ "repositories": entries })).unwrap()
}

/// A slice part-way through a listing, at `page`, with `listed` ids already seen.
fn listing_at(page: u32, listed: usize) -> Value {
    let ids: Vec<String> = (0..listed).map(|id| id.to_string()).collect();
    json!({ "v": 1, "phase": "list", "page": page, "listed": ids, "pending": [], "seen": {} })
}

fn read_one(state: &Value, body: &[u8]) -> Normalised<Record> {
    let request = next_request(state).unwrap();
    normalise(&request, &Response { status: 200, body }, state)
}

fn assert_stopped_clean(out: &Normalised<Record>) {
    assert_eq!(
        out.problems.iter().map(|p| p.code).collect::<Vec<_>>(),
        ["listing_too_long"]
    );
    assert_eq!(
        out.state["listed"],
        json!([]),
        "the slice holds no listed ids"
    );
    assert!(
        !out.records
            .iter()
            .any(|r| matches!(r, Record::Listed { .. })),
        "an unfinished listing never reads as complete"
    );
    assert_eq!(next_request(&out.state), None, "the run stops");
    let next = begin(&out.state);
    assert_eq!(
        next_request(&next).map(|r| query(&r, "page").to_owned()),
        Some("1".to_owned())
    );
}

#[test]
fn src_007_a_listing_past_a_thousand_pages_stops_and_starts_clean() {
    let state = listing_at(1000, 99_900);

    let out = read_one(&state, &full_page_without_a_total(200_000));

    assert_stopped_clean(&out);
}

#[test]
fn src_007_a_listing_past_a_hundred_thousand_repositories_stops_and_starts_clean() {
    let state = listing_at(3, 99_950);

    let out = read_one(&state, &full_page_without_a_total(200_000));

    assert_stopped_clean(&out);
}

#[test]
fn src_007_a_listing_below_the_cap_moves_to_the_next_page() {
    let state = listing_at(999, 99_800);

    let out = read_one(&state, &full_page_without_a_total(200_000));

    assert_eq!(out.problems.len(), 0);
    assert_eq!(out.state["page"], 1000);
    assert_eq!(out.state["listed"].as_array().unwrap().len(), 99_900);
}
