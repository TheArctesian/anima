//! Fixed type identifiers, schema defaults and range checks (CHK-009, DB-011).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{PVE_01, PVE_01_PING, UPTIME_TABLES, at, constraint_of, ensure, id, insert_check};

/// Settings at and beyond each bound: (column, accepted, refused).
const RANGES: [(&str, [i32; 2], [i32; 2]); 5] = [
    ("interval_seconds", [10, 86_400], [9, 86_401]),
    ("retries", [0, 10], [-1, 11]),
    ("retry_interval_seconds", [5, 3_600], [4, 3_601]),
    ("degraded_threshold_ms", [1, 600_000], [0, 600_001]),
    ("failures", [0, 1_000_000], [-1, -1]),
];

const TYPES: [&str; 7] = ["icmp", "tcp", "tls", "http", "dns", "push", "group"];

async fn set(pool: &PgPool, column: &str, value: i32) -> Result<(), sqlx::Error> {
    sqlx::query(&format!(
        "update checks.checks set {column} = $1 where id = $2"
    ))
    .bind(value)
    .bind(id(PVE_01_PING))
    .execute(pool)
    .await
    .map(|_| ())
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn chk_009_fixture_checks_load_with_lower_case_type_ids(pool: PgPool) {
    let types: Vec<(String, i64)> =
        sqlx::query_as("select type, count(*) from checks.checks group by type order by type")
            .fetch_all(&pool)
            .await
            .unwrap();
    let config: Value = sqlx::query_scalar("select config from checks.checks where id = $1")
        .bind(id(PVE_01_PING))
        .fetch_one(&pool)
        .await
        .unwrap();

    let expected = [("dns", 1), ("http", 3), ("icmp", 13), ("tcp", 11)];
    let expected: Vec<(String, i64)> = expected.map(|(t, n)| (t.to_owned(), n)).into();
    assert_eq!(types, expected);
    assert_eq!(config, json!({ "timeoutMs": 2000, "ipVersion": "V4" }));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn db_011_a_new_check_gets_every_default_from_the_schema(pool: PgPool) {
    let check = Uuid::now_v7();
    insert_check(&pool, (check, PVE_01), ("tcp", "SSH"), None)
        .await
        .unwrap();

    let row = sqlx::query(
        "select interval_seconds, retries, retry_interval_seconds, degraded_threshold_ms, \
         invert, paused, config, status, failures, created_at, updated_at, \
         next_due_at is null and last_beat_at is null and cert_not_after is null \
         and resolved_ip is null and secrets_sealed is null and parent_id is null as unset, \
         created_at <= now() as created_by_now \
         from checks.checks where id = $1",
    )
    .bind(check)
    .fetch_one(&pool)
    .await
    .unwrap();

    let settings: [i32; 4] = [0, 1, 2, 3].map(|i| row.get(i));
    assert_eq!(settings, [60, 3, 20, 1000]);
    assert_eq!([row.get::<bool, _>(4), row.get(5)], [false, false]);
    assert_eq!(row.get::<Value, _>(6), json!({}));
    assert_eq!(row.get::<String, _>(7), "unknown");
    assert_eq!(row.get::<i32, _>(8), 0);
    let created: DateTime<Utc> = row.get(9);
    assert_eq!(created, row.get::<DateTime<Utc>, _>(10));
    assert!(row.get::<bool, _>("unset"));
    assert!(row.get::<bool, _>("created_by_now"));
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn db_011_a_new_uptime_bucket_starts_with_zero_counts(pool: PgPool) {
    for table in UPTIME_TABLES {
        let row = sqlx::query(&format!(
            "insert into checks.{table} (check_id, bucket) values ($1, $2) \
             returning up_count, down_count, degraded_count, maintenance_count, \
             unknown_count, response_count, avg_response_ms"
        ))
        .bind(id(PVE_01_PING))
        .bind(at("2030-01-01T00:00:00Z"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let counts: [i32; 6] = [0, 1, 2, 3, 4, 5].map(|i| row.get(i));
        assert_eq!(counts, [0; 6], "{table}");
        assert_eq!(row.get::<Option<f32>, _>(6), None, "{table}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn db_011_check_settings_outside_their_ranges_are_refused(pool: PgPool) {
    for (column, accepted, refused) in RANGES {
        for value in accepted {
            set(&pool, column, value).await.unwrap();
        }
        for value in refused {
            let error = set(&pool, column, value).await.unwrap_err();
            let constraint = format!("checks_{column}_check");
            assert_eq!(
                constraint_of(&error),
                Some(constraint.as_str()),
                "{column} {value}"
            );
        }
    }
    let refusals = [
        ("status = 'late'", "checks_status_check"),
        ("config = '[]'", "checks_config_check"),
        ("name = ''", "checks_name_check"),
    ];
    for (assignment, constraint) in refusals {
        let error = sqlx::query(&format!(
            "update checks.checks set {assignment} where id = $1"
        ))
        .bind(id(PVE_01_PING))
        .execute(&pool)
        .await
        .unwrap_err();
        assert_eq!(constraint_of(&error), Some(constraint), "{assignment}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn db_008_heartbeat_and_uptime_values_outside_their_ranges_are_refused(pool: PgPool) {
    let mut conn = pool.acquire().await.unwrap();
    ensure(&mut conn, at("2030-01-01T00:00:00Z")).await.unwrap();
    let long = Some("é".repeat(513));
    let beats = [
        ("'late'", "null", None, "heartbeats_status_check"),
        ("'up'", "-1", None, "heartbeats_response_time_ms_check"),
        ("'up'", "1", long, "heartbeats_message_check"),
    ];
    for (status, time, message, constraint) in beats {
        let error = sqlx::query(&format!(
            "insert into checks.heartbeats (check_id, at, status, response_time_ms, message) \
             values ($1, '2030-01-01T00:00:00Z', {status}, {time}, $2)"
        ))
        .bind(Uuid::now_v7())
        .bind(message)
        .execute(&mut *conn)
        .await
        .unwrap_err();
        assert_eq!(constraint_of(&error), Some(constraint), "{constraint}");
    }
    sqlx::query(
        "insert into checks.heartbeats (check_id, at, status, message) \
         values ($1, '2030-01-01T00:00:00Z', 'down', $2)",
    )
    .bind(Uuid::now_v7())
    .bind("é".repeat(512))
    .execute(&mut *conn)
    .await
    .unwrap();

    for table in UPTIME_TABLES {
        for column in [
            "up_count",
            "unknown_count",
            "response_count",
            "avg_response_ms",
        ] {
            let error = sqlx::query(&format!(
                "insert into checks.{table} (check_id, bucket, {column}) values ($1, now(), -1)"
            ))
            .bind(id(PVE_01_PING))
            .execute(&pool)
            .await
            .unwrap_err();
            assert_eq!(
                error.as_database_error().unwrap().code().unwrap(),
                "23514",
                "{column}"
            );
        }
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_009_only_the_fixed_lower_case_type_ids_are_accepted(pool: PgPool) {
    for kind in TYPES {
        let token = (kind == "push").then_some("push-token-0123456789abcdef");
        insert_check(&pool, (Uuid::now_v7(), PVE_01), (kind, kind), token)
            .await
            .unwrap();
    }
    for kind in ["ICMP", "docker", "", "tcp "] {
        let error = insert_check(&pool, (Uuid::now_v7(), PVE_01), (kind, "x"), None)
            .await
            .unwrap_err();
        assert_eq!(constraint_of(&error), Some("checks_type_check"), "{kind:?}");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn chk_009_a_push_token_is_set_exactly_for_push_checks(pool: PgPool) {
    let token = Some("push-token-0123456789abcdef");
    let push = insert_check(&pool, (Uuid::now_v7(), PVE_01), ("push", "a"), None).await;
    let tcp = insert_check(&pool, (Uuid::now_v7(), PVE_01), ("tcp", "b"), token).await;
    insert_check(&pool, (Uuid::now_v7(), PVE_01), ("push", "c"), token)
        .await
        .unwrap();
    let reused = insert_check(&pool, (Uuid::now_v7(), PVE_01), ("push", "d"), token).await;
    let renamed = insert_check(&pool, (Uuid::now_v7(), PVE_01), ("tcp", "c"), None).await;

    assert_eq!(
        constraint_of(&push.unwrap_err()),
        Some("checks_push_token_check")
    );
    assert_eq!(
        constraint_of(&tcp.unwrap_err()),
        Some("checks_push_token_check")
    );
    assert_eq!(
        constraint_of(&reused.unwrap_err()),
        Some("checks_push_token_key")
    );
    assert_eq!(
        constraint_of(&renamed.unwrap_err()),
        Some("checks_device_id_name_key")
    );
}
