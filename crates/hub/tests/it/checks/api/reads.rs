//! `Query.check` and the fields of `Check` (CHK-004, CHK-006, CHK-007, CHK-011, API-006).

use anima_hub::{checks, status::Status};
use chrono::TimeDelta;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    PVE_01, beat, create, data_with, database_now, error_json, input, now, record, schema_at,
    schema_for, validation_fields, with_variables,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

fn tcp() -> Value {
    json!({ "tcp": { "port": 22 } })
}

fn days_left(value: &Value) -> &Value {
    &value["check"]["certificateDaysLeft"]
}

const DAYS: &str = "query($id: ID!) { check(id: $id) { certificateDaysLeft } }";
const BEATS: &str = "query($id: ID!, $first: Int, $after: String) { check(id: $id) { \
    heartbeats(first: $first, after: $after) { edges { cursor node { at status responseTimeMs } } \
    pageInfo { hasNextPage endCursor } } } }";

async fn set_expiry(pool: &PgPool, id: &str, offset: TimeDelta) {
    sqlx::query("UPDATE checks.checks SET cert_not_after = $2 WHERE id = $1::text::uuid")
        .bind(id)
        .bind(now() + offset)
        .execute(pool)
        .await
        .unwrap();
}

fixture_test! {
    async fn api_003_check_is_null_for_an_unknown_id_and_validation_for_a_malformed_one(pool: PgPool) {
        let schema = schema_for(&pool);
        let query = "query($id: ID!) { check(id: $id) { id } }";
        let unknown = "0190a000-0000-7000-8003-0000000000ff";
        assert_eq!(data_with(&schema, query, json!({ "id": unknown })).await["check"], Value::Null);
        let error = error_json(&with_variables(&schema, query, json!({ "id": "x" })).await);
        assert_eq!(validation_fields(&error), ["id"]);
    }

    async fn chk_007_certificate_days_left_floors_and_goes_negative_when_expired(pool: PgPool) {
        let schema = schema_for(&pool);
        let tls = create(&schema, input(PVE_01, "tls", json!({ "tls": {} }))).await;
        let cases = [
            (TimeDelta::days(10) + TimeDelta::hours(1), 10),
            (TimeDelta::hours(23), 0),
            (TimeDelta::zero(), 0),
            (-TimeDelta::hours(1), -1),
            (-TimeDelta::days(3), -3),
        ];
        for (offset, expected) in cases {
            set_expiry(&pool, &tls, offset).await;
            let read = data_with(&schema, DAYS, json!({ "id": tls })).await;
            assert_eq!(days_left(&read), &json!(expected), "{offset}");
        }
    }

    async fn chk_007_certificate_days_left_is_null_without_a_certificate_or_for_other_types(pool: PgPool) {
        let schema = schema_for(&pool);
        let tls = create(&schema, input(PVE_01, "tls", json!({ "tls": {} }))).await;
        let secure = create(&schema, input(PVE_01, "https", json!({ "http": { "url": "https://web.lan/" } }))).await;
        let plain = create(&schema, input(PVE_01, "http", json!({ "http": { "url": "http://web.lan/" } }))).await;
        let port = create(&schema, input(PVE_01, "tcp", tcp())).await;
        let read = data_with(&schema, DAYS, json!({ "id": tls })).await;
        assert_eq!(days_left(&read), &Value::Null, "no certificate seen yet");
        set_expiry(&pool, &secure, TimeDelta::days(5)).await;
        set_expiry(&pool, &plain, TimeDelta::days(5)).await;
        set_expiry(&pool, &port, TimeDelta::days(5)).await;
        let secure = data_with(&schema, DAYS, json!({ "id": secure })).await;
        assert_eq!(days_left(&secure), &json!(5));
        for id in [plain, port] {
            let read = data_with(&schema, DAYS, json!({ "id": id })).await;
            assert_eq!(days_left(&read), &Value::Null);
        }
    }

    async fn api_006_heartbeats_page_newest_first_with_a_keyset_cursor(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        let base = database_now(&pool).await;
        for seconds in (0..5).rev() {
            let at = base - TimeDelta::seconds(seconds);
            record(&pool, &id, &beat(at, Status::Up, Some(u32::try_from(seconds).unwrap()))).await;
        }
        let first = data_with(&schema, BEATS, json!({ "id": id, "first": 2 })).await;
        let page = &first["check"]["heartbeats"];
        let times: Vec<i64> = page["edges"].as_array().unwrap().iter()
            .map(|e| e["node"]["responseTimeMs"].as_i64().unwrap()).collect();
        assert_eq!(times, [0, 1]);
        assert_eq!(page["pageInfo"]["hasNextPage"], true);
        let cursor = page["pageInfo"]["endCursor"].as_str().unwrap();
        assert_eq!(page["edges"][1]["cursor"], cursor);

        let next = data_with(&schema, BEATS, json!({ "id": id, "first": 2, "after": cursor })).await;
        let page = &next["check"]["heartbeats"];
        let times: Vec<i64> = page["edges"].as_array().unwrap().iter()
            .map(|e| e["node"]["responseTimeMs"].as_i64().unwrap()).collect();
        assert_eq!(times, [2, 3]);
        let cursor = page["pageInfo"]["endCursor"].as_str().unwrap();

        let last = data_with(&schema, BEATS, json!({ "id": id, "first": 2, "after": cursor })).await;
        let page = &last["check"]["heartbeats"];
        assert_eq!(page["edges"].as_array().unwrap().len(), 1);
        assert_eq!(page["pageInfo"]["hasNextPage"], false);

        let all = data_with(&schema, BEATS, json!({ "id": id })).await;
        assert_eq!(all["check"]["heartbeats"]["edges"].as_array().unwrap().len(), 5);
    }

    async fn api_006_has_next_page_is_false_when_exactly_a_page_remains(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        let base = database_now(&pool).await;
        for seconds in (0..4).rev() {
            record(&pool, &id, &beat(base - TimeDelta::seconds(seconds), Status::Up, Some(1))).await;
        }
        let first = data_with(&schema, BEATS, json!({ "id": id, "first": 2 })).await;
        let page = &first["check"]["heartbeats"]["pageInfo"];
        assert_eq!(page["hasNextPage"], true);
        let cursor = page["endCursor"].as_str().unwrap();
        let second = data_with(&schema, BEATS, json!({ "id": id, "first": 2, "after": cursor })).await;
        let last = &second["check"]["heartbeats"];
        assert_eq!(last["edges"].as_array().unwrap().len(), 2);
        assert_eq!(last["pageInfo"]["hasNextPage"], false, "exactly one page remained");
        let all = data_with(&schema, BEATS, json!({ "id": id, "first": 4 })).await;
        assert_eq!(all["check"]["heartbeats"]["pageInfo"]["hasNextPage"], false);
    }

    async fn api_006_checks_paged_together_keep_their_own_pages_and_cursors(pool: PgPool) {
        let schema = schema_for(&pool);
        let long = create(&schema, input(PVE_01, "long", tcp())).await;
        let short = create(&schema, input(PVE_01, "short", json!({ "tcp": { "port": 23 } }))).await;
        let base = database_now(&pool).await;
        for seconds in (0..5).rev() {
            let at = base - TimeDelta::seconds(seconds);
            record(&pool, &long, &beat(at, Status::Up, Some(u32::try_from(seconds).unwrap()))).await;
        }
        for seconds in (0..3).rev() {
            let at = base - TimeDelta::seconds(seconds);
            record(&pool, &short, &beat(at, Status::Down, None)).await;
        }
        let query = "query($a: ID!, $b: ID!, $ca: String, $cb: String) { \
            a: check(id: $a) { heartbeats(first: 2, after: $ca) { edges { node { status responseTimeMs } } \
              pageInfo { hasNextPage endCursor } } } \
            b: check(id: $b) { heartbeats(first: 2, after: $cb) { edges { node { status responseTimeMs } } \
              pageInfo { hasNextPage endCursor } } } }";
        let first = data_with(&schema, query, json!({ "a": long, "b": short })).await;
        let (a, b) = (&first["a"]["heartbeats"], &first["b"]["heartbeats"]);
        assert_eq!(a["edges"].as_array().unwrap().len(), 2);
        assert_eq!(a["edges"][0]["node"]["responseTimeMs"], 0);
        assert_eq!((a["pageInfo"]["hasNextPage"].clone(), b["pageInfo"]["hasNextPage"].clone()), (json!(true), json!(true)));
        // Each check continues from its own cursor, in the same request.
        let vars = json!({ "a": long, "b": short,
            "ca": a["pageInfo"]["endCursor"], "cb": b["pageInfo"]["endCursor"] });
        let second = data_with(&schema, query, vars).await;
        let (a, b) = (&second["a"]["heartbeats"], &second["b"]["heartbeats"]);
        assert_eq!(a["edges"].as_array().unwrap().len(), 2);
        assert_eq!(a["edges"][0]["node"]["responseTimeMs"], 2);
        assert_eq!(a["pageInfo"]["hasNextPage"], true);
        assert_eq!(b["edges"].as_array().unwrap().len(), 1);
        assert_eq!(b["edges"][0]["node"]["status"], "DOWN");
        assert_eq!(b["pageInfo"]["hasNextPage"], false);
    }

    async fn api_006_a_cursor_outside_the_stored_range_is_a_validation_error(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        for cursor in ["h1.-300000000000000000", "h1.999999999999999999", "h1.-1"] {
            let vars = json!({ "id": id, "after": cursor });
            let error = error_json(&with_variables(&schema, BEATS, vars).await);
            assert_eq!(validation_fields(&error), ["after"], "{cursor}");
        }
    }

    async fn api_006_heartbeats_refuse_a_page_size_outside_1_to_500_and_a_bad_cursor(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        for first in [0, 501, -1] {
            let error = error_json(&with_variables(&schema, BEATS, json!({ "id": id, "first": first })).await);
            assert_eq!(validation_fields(&error), ["first"], "{first}");
        }
        data_with(&schema, BEATS, json!({ "id": id, "first": 500 })).await;
        let bad = json!({ "id": id, "after": "n1.zz" });
        let error = error_json(&with_variables(&schema, BEATS, bad).await);
        assert_eq!(validation_fields(&error), ["after"]);
    }

    async fn chk_003_last_heartbeat_is_the_newest_and_null_before_the_first(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        let query = "query($id: ID!) { check(id: $id) { lastHeartbeat { status responseTimeMs } } }";
        let none = data_with(&schema, query, json!({ "id": id })).await;
        assert_eq!(none["check"]["lastHeartbeat"], Value::Null);
        let at = database_now(&pool).await;
        record(&pool, &id, &beat(at - TimeDelta::seconds(5), Status::Down, None)).await;
        record(&pool, &id, &beat(at, Status::Up, Some(7))).await;
        let read = data_with(&schema, query, json!({ "id": id })).await;
        assert_eq!(read["check"]["lastHeartbeat"], json!({ "status": "UP", "responseTimeMs": 7 }));
    }

    async fn chk_011_uptime_of_a_check_comes_from_its_aggregates(pool: PgPool) {
        let at = database_now(&pool).await;
        let schema = schema_at(&pool, at).0;
        let id = create(&schema, input(PVE_01, "ssh", tcp())).await;
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [] } }))).await;
        let query = "query($id: ID!) { check(id: $id) { uptime(window: DAY) } }";
        let none = data_with(&schema, query, json!({ "id": id })).await;
        assert_eq!(none["check"]["uptime"], Value::Null);
        for (seconds, status) in [(300, Status::Up), (240, Status::Up), (180, Status::Degraded), (120, Status::Down)] {
            record(&pool, &id, &beat(at - TimeDelta::seconds(seconds), status, Some(5))).await;
        }
        let read = data_with(&schema, query, json!({ "id": id })).await;
        assert_eq!(read["check"]["uptime"], json!(0.75));
        let other = data_with(&schema, query, json!({ "id": group })).await;
        assert_eq!(other["check"]["uptime"], Value::Null);
    }

    async fn chk_006_the_displayed_status_agrees_with_the_device_status(pool: PgPool) {
        let at = database_now(&pool).await;
        let schema = schema_at(&pool, at).0;
        let down = create(&schema, input(PVE_01, "down", json!({ "tcp": { "port": 22 } }))).await;
        let up = create(&schema, input(PVE_01, "up", json!({ "tcp": { "port": 23 } }))).await;
        record(&pool, &down, &beat(at, Status::Down, None)).await;
        record(&pool, &up, &beat(at, Status::Up, Some(1))).await;
        let query = "query($id: ID!) { check(id: $id) { status } }";
        let device = PVE_01.parse().unwrap();
        for manual in [false, true] {
            sqlx::query("UPDATE devices.devices SET maintenance = $1 WHERE id = $2::text::uuid")
                .bind(manual)
                .bind(PVE_01)
                .execute(&pool)
                .await
                .unwrap();
            let shown_down = data_with(&schema, query, json!({ "id": down })).await;
            let shown_up = data_with(&schema, query, json!({ "id": up })).await;
            let expected = if manual { "MAINTENANCE" } else { "DOWN" };
            assert_eq!(shown_down["check"]["status"], expected);
            assert_eq!(shown_up["check"]["status"], "UP");
            let overall = checks::device_status(&pool, device, at).await.unwrap();
            assert_eq!(format!("{overall:?}").to_uppercase(), expected);
        }
        let paused = "mutation($id: ID!) { setCheckPaused(id: $id, paused: true) { status } }";
        let shown = data_with(&schema, paused, json!({ "id": down })).await;
        assert_eq!(shown["setCheckPaused"]["status"], "UNKNOWN");
    }

    async fn api_003_device_parent_and_push_path_resolve(pool: PgPool) {
        let schema = schema_for(&pool);
        let push = create(&schema, input(PVE_01, "push", json!({ "push": true }))).await;
        let query = "query($id: ID!) { check(id: $id) { device { name } parent { id } \
            config { ... on PushConfig { pushPath } } } }";
        let read = data_with(&schema, query, json!({ "id": push })).await;
        assert_eq!(read["check"]["device"]["name"], "pve-01");
        assert_eq!(read["check"]["parent"], Value::Null);
        let path = read["check"]["config"]["pushPath"].as_str().unwrap();
        let token = path.strip_prefix("/push/").unwrap();
        assert_eq!(token.len(), 43);
        assert!(token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        let other = create(&schema, input(PVE_01, "push2", json!({ "push": true }))).await;
        let second = data_with(&schema, query, json!({ "id": other })).await;
        assert_ne!(second["check"]["config"]["pushPath"], read["check"]["config"]["pushPath"]);
    }
}
