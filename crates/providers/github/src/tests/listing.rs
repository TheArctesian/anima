//! SRC-007: a listing with no end does not grow the stored slice without limit. A listed
//! repository's text cannot hold U+0000, which no PostgreSQL text or JSON value takes.

use super::*;

#[test]
fn src_011_u0000_is_taken_out_of_every_field_and_the_original() {
    let mut entry = repository(7, "arctesian/n\u{0}ul", "2026-10-01T08:00:00Z");
    entry["description"] = json!("half\u{0}way\u{0}");
    entry["owner"] = json!({ "login": "arc\u{0}tesian", "tags": ["a\u{0}", 1, null] });
    entry["we\u{0}ird"] = json!("v\u{0}");
    let body = listing(1, &[entry]);

    let out = read_one(&begin(&json!({})), &body);

    let Some(Record::Repository(repository)) = out.records.first() else {
        panic!("no repository: {:?}", out.records);
    };
    assert_eq!(repository.description.as_deref(), Some("halfway"));
    assert_eq!(repository.full_name, "arctesian/nul");
    assert_eq!(repository.original["description"], "halfway");
    assert_eq!(
        repository.original["owner"],
        json!({ "login": "arctesian", "tags": ["a", 1, null] })
    );
    assert_eq!(repository.original["weird"], "v");
    let text = serde_json::to_string(&repository.original).unwrap();
    assert!(!text.contains("\\u0000"), "{text}");
}

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
    normalise(
        &request,
        &Response {
            status: 200,
            body: Body::Read(body),
        },
        state,
    )
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
