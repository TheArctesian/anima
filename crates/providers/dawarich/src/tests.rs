//! The pure functions of the Dawarich provider (SRC-006, SRC-012; K-E4 Section 11).

use serde_json::{Value, json};
use source_exchange::{Acquisition, Body, Response};

use super::*;

fn reply(body: &[u8]) -> Response<'_> {
    Response {
        status: 200,
        body: Body::Read(body),
        link: None,
    }
}

fn page(points: &Value, state: &Value) -> Normalised<Point> {
    let body = points.to_string();
    let request = next_request(state).unwrap();
    normalise(&request, &reply(body.as_bytes()), state)
}

#[test]
fn src_001_the_declaration_is_read_only_and_has_a_key_of_the_account() {
    assert_eq!(DECLARATION.key, "dawarich");
    assert!(DECLARATION.write_reason.is_none() && DECLARATION.changes.is_none());
    assert!(DECLARATION.poll_interval_is_allowed());
    let Acquisition::AccountKey(key) = DECLARATION.acquisition else {
        panic!("Dawarich connects with a key of the account");
    };
    let probe = (key.probe)();
    assert_eq!(probe.path, "/api/v1/points");
    assert_eq!(probe.query, [("per_page".to_owned(), "1".to_owned())]);
}

#[test]
fn r_145_the_first_run_asks_for_each_point_from_the_start_in_order() {
    let state = begin(&json!({}));
    let request = next_request(&state).unwrap();
    assert_eq!(request.path, "/api/v1/points");
    let query: Vec<(&str, &str)> = request
        .query
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    assert_eq!(
        query,
        [
            ("start_at", "1970-01-01T00:00:00Z"),
            ("end_at", "2100-01-01T00:00:00Z"),
            ("order", "asc"),
            ("page", "1"),
            ("per_page", "1000"),
        ]
    );
}

#[test]
fn r_145_a_point_keeps_its_id_its_time_and_a_missing_number() {
    let points = json!([
        { "id": 41, "timestamp": 1_794_830_400, "latitude": "52.1", "longitude": 4.2,
          "accuracy": 12, "altitude": null, "velocity": "36" },
        { "id": 42, "timestamp": 1_794_830_460, "latitude": "52.2", "longitude": "4.3" },
    ]);
    let read = page(&points, &begin(&json!({})));
    assert_eq!(read.problems, []);
    assert_eq!(
        read.records[0],
        Point {
            id: "41".to_owned(),
            timestamp: Some(1_794_830_400),
            latitude: Some(52.1),
            longitude: Some(4.2),
            accuracy_m: Some(12.0),
            altitude_m: None,
            speed_mps: Some(10.0),
        }
    );
    assert_eq!(read.records[1].accuracy_m, None, "no number in its place");
    assert_eq!(read.state["done"], true);
    assert_eq!(read.state["since"], 1_794_830_460_i64);
    assert_eq!(next_request(&read.state), None);
}

#[test]
fn src_007_a_full_page_asks_for_the_next_one_and_keeps_the_start_of_the_run() {
    let full: Vec<Value> = (0..PER_PAGE)
        .map(|n| json!({ "id": n, "timestamp": 1_794_830_400 + n, "latitude": 1, "longitude": 2 }))
        .collect();
    let state = begin(&json!({ "v": 1, "since": 1_794_000_000_i64 }));
    let first = page(&Value::Array(full), &state);
    assert_eq!(first.records.len(), PER_PAGE);
    assert_eq!(
        (first.state["done"].clone(), first.state["page"].clone()),
        (json!(false), json!(2))
    );
    assert_eq!(
        first.state["since"], 1_794_000_000_i64,
        "the start moves when the run ends"
    );
    let next = next_request(&first.state).unwrap();
    assert!(next.query.contains(&("page".to_owned(), "2".to_owned())));
    assert!(
        next.query
            .contains(&("start_at".to_owned(), "2026-11-06T21:19:59Z".to_owned()))
    );

    let last = page(&json!([]), &first.state);
    assert_eq!(last.state["done"], true);
    assert_eq!(last.state["since"], 1_794_830_400_i64 + 999);
    // The next run starts one second before the last point.
    let again = next_request(&begin(&last.state)).unwrap();
    assert!(
        again
            .query
            .contains(&("start_at".to_owned(), "2026-11-16T12:16:38Z".to_owned()))
    );
}

#[test]
fn src_012_the_same_page_again_gives_the_same_records_and_state() {
    let points = json!([{ "id": 1, "timestamp": 10, "latitude": 1, "longitude": 2 }]);
    let state = begin(&json!({}));
    let (first, second) = (page(&points, &state), page(&points, &state));
    assert_eq!(first.records, second.records);
    assert_eq!(first.state, second.state);
}

#[test]
fn src_006_a_reply_that_is_no_page_keeps_the_state_and_says_why() {
    let state = begin(&json!({}));
    let request = next_request(&state).unwrap();
    let refused = Response {
        status: 500,
        body: Body::Read(b"secret reply text"),
        link: None,
    };
    let read = normalise(&request, &refused, &state);
    assert_eq!(read.state, state);
    assert_eq!(read.problems[0].code, "status");
    assert!(!read.problems[0].message.contains("secret"));
    assert_eq!(
        normalise(&request, &reply(b"{}"), &state).problems[0].code,
        "form"
    );
    let large = Response {
        status: 200,
        body: Body::TooLarge,
        link: None,
    };
    assert_eq!(
        normalise(&request, &large, &state).problems[0].code,
        "too_large"
    );
}

#[test]
fn r_145_a_point_with_no_id_is_a_problem_and_not_a_record() {
    let points = json!([{ "timestamp": 10, "latitude": 1, "longitude": 2 }, { "id": "7" }]);
    let read = page(&points, &begin(&json!({})));
    assert_eq!(read.records.len(), 1);
    assert_eq!(read.records[0].id, "7");
    assert_eq!(read.problems[0].code, "no_id");
}
