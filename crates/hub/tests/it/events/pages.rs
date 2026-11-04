//! Cursor pages of both lists: walking them, and refusing bad sizes, cursors and ids or a
//! signed-out caller (API-006, API-003).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::hub::{Hub, set_operator, start, unauthenticated};

pub const EVENTS: &str = "query($first: Int, $after: String, $filter: EventFilter) { \
    events(first: $first, after: $after, filter: $filter) { \
    edges { cursor node { id at severity source deviceId deviceName message } } \
    pageInfo { hasNextPage endCursor } } }";

/// The event list without the sign-in events `login` records.
pub const HUB_EVENTS: &str = "query($first: Int, $after: String) { \
    events(first: $first, after: $after, filter: { source: [HUB] }) { \
    edges { cursor node { id } } pageInfo { hasNextPage endCursor } } }";

pub const AUDIT: &str = "query($first: Int, $after: String) { \
    audit(first: $first, after: $after) { \
    edges { cursor node { id at operatorName action targetType targetId clientAddr change } } \
    pageInfo { hasNextPage endCursor } } }";

/// The instant a K1 `DateTime` names, whatever RFC 3339 offset it is written with (R-033).
pub fn instant(value: &Value) -> DateTime<Utc> {
    let text = value
        .as_str()
        .unwrap_or_else(|| panic!("not a DateTime: {value}"));
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// Walks every page of `query` (whose root field is `list`) at `first` entries a page; returns
/// the node ids in order and the `hasNextPage` of each page. Each page's `endCursor` must be
/// its last edge's cursor, and a walk of more than 20 pages fails.
pub async fn walk(
    hub: &Hub,
    token: &str,
    (query, list): (&str, &str),
    first: i32,
) -> (Vec<String>, Vec<bool>) {
    let (mut ids, mut more, mut after) = (Vec::new(), Vec::new(), Value::Null);
    loop {
        let variables = json!({ "first": first, "after": after });
        let response = hub.post(Some(token), query, variables).await;
        let page = &response["data"][list];
        let edges = page["edges"]
            .as_array()
            .unwrap_or_else(|| panic!("{response}"));
        let node_id = |edge: &Value| edge["node"]["id"].as_str().unwrap().to_owned();
        ids.extend(edges.iter().map(node_id));
        let last_cursor = edges.last().map(|edge| edge["cursor"].clone());
        assert_eq!(
            page["pageInfo"]["endCursor"],
            last_cursor.unwrap_or(Value::Null)
        );
        let has_next_page = page["pageInfo"]["hasNextPage"].as_bool().unwrap();
        more.push(has_next_page);
        assert!(more.len() <= 20, "the pages never end: {ids:?}");
        if !has_next_page {
            return (ids, more);
        }
        after = page["pageInfo"]["endCursor"].clone();
    }
}

/// The fields a `VALIDATION` error names; the response carries no data.
fn fields(response: &Value) -> Vec<String> {
    assert_eq!(
        response["errors"][0]["extensions"]["code"], "VALIDATION",
        "{response}"
    );
    assert!(response["data"].is_null(), "{response}");
    response["errors"][0]["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["field"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_bad_page_sizes_cursors_and_device_ids_are_refused(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let audit_page = hub.post(Some(&token), AUDIT, json!({})).await;
    let audit_cursor = audit_page["data"]["audit"]["pageInfo"]["endCursor"].clone();
    assert!(audit_cursor.is_string(), "{audit_page}");
    let bad_device = json!({ "deviceId": "lab-router" });
    for (query, variables, expected) in [
        (EVENTS, json!({ "first": 0 }), vec!["first"]),
        (EVENTS, json!({ "first": 201 }), vec!["first"]),
        (EVENTS, json!({ "first": -1 }), vec!["first"]),
        (EVENTS, json!({ "after": "garbage" }), vec!["after"]),
        (EVENTS, json!({ "after": audit_cursor }), vec!["after"]),
        (
            EVENTS,
            json!({ "filter": bad_device }),
            vec!["filter.deviceId"],
        ),
        (
            EVENTS,
            json!({ "first": 0, "filter": bad_device }),
            vec!["first", "filter.deviceId"],
        ),
        (AUDIT, json!({ "first": 0 }), vec!["first"]),
        (AUDIT, json!({ "first": 201 }), vec!["first"]),
        (
            AUDIT,
            json!({ "after": "e1.0.0192f5a07c3e7a529d1b3f4e5a6b7c8d" }),
            vec!["after"],
        ),
    ] {
        let response = hub.post(Some(&token), query, variables.clone()).await;
        assert_eq!(fields(&response), expected, "{variables}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_both_lists_need_sign_in(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for query in [EVENTS, AUDIT] {
        let response = hub.post(None, query, json!({})).await;
        assert_eq!(crate::hub::error(&response), unauthenticated(), "{query}");
    }
}
