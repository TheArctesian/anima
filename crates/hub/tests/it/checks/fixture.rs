//! The checks fixture holds exactly the K6 checks, with lower-case type ids (CHK-009).

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// One check as (id, device id, name, type, config).
type Row = (Uuid, Uuid, String, String, Value);

fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}

/// The K6 checks, with each GraphQL type in its stored form and the config variant unwrapped.
fn k6_checks() -> Vec<Row> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/fixtures/phase1.json"
    );
    let phase1: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut checks: Vec<Row> = phase1["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| {
            let kind = text(&check["type"]).to_lowercase();
            let config = check["config"][kind.as_str()].clone();
            assert!(config.is_object(), "{check}");
            (
                text(&check["id"]).parse().unwrap(),
                text(&check["deviceId"]).parse().unwrap(),
                text(&check["name"]).to_owned(),
                kind,
                config,
            )
        })
        .collect();
    checks.sort_by_key(|check| check.0);
    checks
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices", "checks"))
)]
async fn chk_009_fixture_checks_are_the_k6_checks_with_lower_case_type_ids(pool: PgPool) {
    let stored: Vec<Row> =
        sqlx::query_as("select id, device_id, name, type, config from checks.checks order by id")
            .fetch_all(&pool)
            .await
            .unwrap();

    let expected = k6_checks();
    assert_eq!(expected.len(), 28);
    assert_eq!(stored, expected);
}
