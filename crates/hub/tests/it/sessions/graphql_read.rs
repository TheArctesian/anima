//! `Query.sessions`: fields, filters and keyset paging (RA-008, RA-034, API-006, K7).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    DOCKER_HOST, PVE_01, Row, delete_device,
    harness::{data, only_error, schema_for},
    insert, insert_all, session_id,
};

const FIELDS: &str =
    "id deviceId deviceName protocol operatorName startedAt endedAt endReason clientAddr";

fn ids(page: &Value) -> Vec<String> {
    page["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["id"].as_str().unwrap().to_owned())
        .collect()
}

fn id(n: u128) -> String {
    session_id(n).to_string()
}

/// The node with its times rewritten as `…Z`, so the test checks the instant. The hub's
/// `DateTime` scalar (async-graphql's, R-012) writes the offset as `+00:00`.
fn instants_as_utc_z(node: &Value) -> Value {
    let mut node = node.clone();
    for field in ["startedAt", "endedAt"] {
        if let Some(text) = node[field].as_str() {
            let at: DateTime<Utc> = text.parse().unwrap();
            node[field] = json!(at.to_rfc3339_opts(SecondsFormat::Secs, true));
        }
    }
    node
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_a_session_shows_the_k1_fields_and_outlives_its_device(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::ended(1, PVE_01, 0, "host_key_mismatch"),
            Row::live(2, DOCKER_HOST, 10),
        ],
    )
    .await;
    sqlx::query("update sessions.sessions set protocol = 'vnc' where id = $1")
        .bind(session_id(2))
        .execute(&pool)
        .await
        .unwrap();
    delete_device(&pool, PVE_01).await;

    let schema = schema_for(&pool);
    let page = data(
        &schema,
        &format!("{{ sessions {{ edges {{ node {{ {FIELDS} }} }} }} }}"),
    )
    .await;
    let nodes: Vec<Value> = page["sessions"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| instants_as_utc_z(&edge["node"]))
        .collect();
    assert_eq!(
        nodes,
        [
            json!({
                "id": id(2), "deviceId": DOCKER_HOST, "deviceName": "docker-host",
                "protocol": "VNC", "operatorName": "operator",
                "startedAt": "2026-11-04T09:10:00Z", "endedAt": null, "endReason": null,
                "clientAddr": "10.10.0.77",
            }),
            json!({
                "id": id(1), "deviceId": PVE_01, "deviceName": "pve-01",
                "protocol": "SSH", "operatorName": "operator",
                "startedAt": "2026-11-04T09:00:00Z", "endedAt": "2026-11-04T09:05:00Z",
                "endReason": "HOST_KEY_MISMATCH", "clientAddr": "10.10.0.77",
            }),
        ]
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn ra_008_live_and_device_filters_select_sessions(pool: PgPool) {
    insert_all(
        &pool,
        &[
            Row::live(1, PVE_01, 0),
            Row::ended(2, PVE_01, 1, "operator"),
            Row::live(3, DOCKER_HOST, 2),
            Row::ended(4, DOCKER_HOST, 3, "timeout"),
        ],
    )
    .await;
    let schema = schema_for(&pool);
    let cases = [
        (String::new(), vec![4, 3, 2, 1]),
        ("(live: true)".to_owned(), vec![3, 1]),
        ("(live: false)".to_owned(), vec![4, 2]),
        (format!(r#"(deviceId: "{PVE_01}")"#), vec![2, 1]),
        (
            format!(r#"(deviceId: "{DOCKER_HOST}", live: true)"#),
            vec![3],
        ),
        (
            format!(r#"(deviceId: "{DOCKER_HOST}", live: null)"#),
            vec![4, 3],
        ),
    ];
    for (args, expected) in cases {
        let page = data(
            &schema,
            &format!("{{ sessions{args} {{ edges {{ node {{ id }} }} }} }}"),
        )
        .await;
        let expected: Vec<String> = expected.into_iter().map(id).collect();
        assert_eq!(ids(&page["sessions"]), expected, "{args}");
    }
}

async fn page(schema: &super::harness::TestSchema, first: i32, after: Option<&str>) -> Value {
    let after = after.map_or_else(String::new, |cursor| format!(r#", after: "{cursor}""#));
    let query = format!(
        "{{ sessions(first: {first}{after}) {{ edges {{ cursor node {{ id }} }} \
           pageInfo {{ hasNextPage endCursor }} }} }}"
    );
    data(schema, &query).await["sessions"].clone()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_006_pages_are_newest_first_and_stable_under_new_sessions(pool: PgPool) {
    // Rows 1-3 start at the same instant, so the id breaks the tie.
    let rows: Vec<Row> = (1..=7)
        .map(|n| Row::live(n, PVE_01, i64::try_from(n.saturating_sub(3)).unwrap()))
        .collect();
    insert_all(&pool, &rows).await;
    let schema = schema_for(&pool);

    let first = page(&schema, 3, None).await;
    assert_eq!(ids(&first), [id(7), id(6), id(5)]);
    assert_eq!(first["pageInfo"]["hasNextPage"], true);
    assert_eq!(first["pageInfo"]["endCursor"], first["edges"][2]["cursor"]);

    // A session that starts now goes before page one and does not shift page two.
    insert(&pool, &Row::live(8, DOCKER_HOST, 60)).await.unwrap();
    let cursor = first["pageInfo"]["endCursor"].as_str().unwrap();
    let second = page(&schema, 3, Some(cursor)).await;
    assert_eq!(ids(&second), [id(4), id(3), id(2)]);
    let cursor = second["pageInfo"]["endCursor"].as_str().unwrap();
    let last = page(&schema, 3, Some(cursor)).await;
    assert_eq!(ids(&last), [id(1)]);
    assert_eq!(
        last["pageInfo"],
        json!({ "hasNextPage": false, "endCursor": last["edges"][0]["cursor"] })
    );

    let empty = page(&schema, 3, last["pageInfo"]["endCursor"].as_str()).await;
    assert_eq!(
        empty,
        json!({ "edges": [], "pageInfo": { "hasNextPage": false, "endCursor": null } })
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_006_first_defaults_to_fifty(pool: PgPool) {
    let rows: Vec<Row> = (1..=51)
        .map(|n| Row::live(n, PVE_01, i64::try_from(n).unwrap()))
        .collect();
    insert_all(&pool, &rows).await;
    let schema = schema_for(&pool);
    let page = data(
        &schema,
        "{ sessions { edges { cursor } pageInfo { hasNextPage } } }",
    )
    .await;
    assert_eq!(page["sessions"]["edges"].as_array().unwrap().len(), 50);
    assert_eq!(page["sessions"]["pageInfo"]["hasNextPage"], true);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_006_bad_arguments_are_a_validation_error_naming_each_one(pool: PgPool) {
    let schema = schema_for(&pool);
    let cases = [
        ("first: 0", vec!["first"]),
        ("first: 201", vec!["first"]),
        ("first: -5", vec!["first"]),
        (r#"after: "n1.6162""#, vec!["after"]),
        (r#"deviceId: "pve-01""#, vec!["deviceId"]),
        (
            r#"deviceId: "x", first: 999, after: "s1.nope""#,
            vec!["deviceId", "first", "after"],
        ),
    ];
    for (args, expected) in cases {
        let error = only_error(
            &schema,
            &format!("{{ sessions({args}) {{ pageInfo {{ hasNextPage }} }} }}"),
        )
        .await;
        assert_eq!(error["extensions"]["code"], "VALIDATION", "{args}");
        let fields: Vec<&str> = error["extensions"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field["field"].as_str().unwrap())
            .collect();
        assert_eq!(fields, expected, "{args}");
    }
    let ok = data(&schema, "{ sessions(first: 200) { edges { cursor } } }").await;
    assert_eq!(ok["sessions"]["edges"], json!([]));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_010_a_row_the_hub_cannot_read_gives_a_generic_internal_error(pool: PgPool) {
    sqlx::query("alter table sessions.sessions drop constraint sessions_end_reason_check")
        .execute(&pool)
        .await
        .unwrap();
    insert(&pool, &Row::ended(1, PVE_01, 0, "secret-reason-text"))
        .await
        .unwrap();
    let error = only_error(&schema_for(&pool), "{ sessions { edges { cursor } } }").await;
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    assert_eq!(
        error["message"],
        "Something went wrong on the hub. Try again."
    );
    assert!(!error.to_string().contains("secret-reason-text"), "{error}");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_010_a_half_ended_row_gives_a_generic_internal_error(pool: PgPool) {
    sqlx::query("alter table sessions.sessions drop constraint sessions_end_pair_check")
        .execute(&pool)
        .await
        .unwrap();
    insert(&pool, &Row::live(1, PVE_01, 0)).await.unwrap();
    sqlx::query("update sessions.sessions set ended_at = started_at where id = $1")
        .bind(session_id(1))
        .execute(&pool)
        .await
        .unwrap();
    let error = only_error(&schema_for(&pool), "{ sessions { edges { cursor } } }").await;
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    assert_eq!(
        error["message"],
        "Something went wrong on the hub. Try again."
    );
    assert!(!error.to_string().contains("ended_at"), "{error}");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_010_a_cursor_time_postgresql_cannot_store_is_a_validation_error(pool: PgPool) {
    // A well-formed cursor at chrono's earliest instant, far before PostgreSQL's 4713 BC.
    let mut bytes = [0_u8; 24];
    bytes[..8].copy_from_slice(&DateTime::<Utc>::MIN_UTC.timestamp_micros().to_be_bytes());
    let cursor = format!("s1.{}", URL_SAFE_NO_PAD.encode(bytes));
    let error = only_error(
        &schema_for(&pool),
        &format!(r#"{{ sessions(after: "{cursor}") {{ edges {{ cursor }} }} }}"#),
    )
    .await;
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    assert_eq!(error["extensions"]["fields"][0]["field"], "after");
}
