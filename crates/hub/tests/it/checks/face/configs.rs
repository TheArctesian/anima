//! Every fixture configuration reads in the one stored form, keeps every value, and writes
//! back the same (CHK-009).

use anima_hub::{
    checks::{self, config::CheckConfig},
    ids::{CheckId, DeviceId},
};
use serde_json::{Map, Value};
use sqlx::PgPool;

/// The stored form of a configuration: its JSON without the type tag.
fn stored(config: &CheckConfig) -> Map<String, Value> {
    let Value::Object(mut fields) = serde_json::to_value(config).unwrap() else {
        panic!("not an object")
    };
    fields.remove("type");
    fields
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices", "checks"))
)]
async fn chk_009_every_fixture_config_reads_and_writes_back_its_values(pool: PgPool) {
    let rows: Vec<(CheckId, DeviceId, String, Value)> =
        sqlx::query_as("SELECT id, device_id, type, config FROM checks.checks ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 28);
    for (id, device, kind, fixture) in rows {
        let check = checks::get(&pool, id).await.unwrap();
        let written = stored(&check.config);
        let Value::Object(fixture) = fixture else {
            panic!("{id}: not an object")
        };
        if kind == "http" {
            // HTTP also writes its defaults; every fixture value is kept.
            for (key, value) in &fixture {
                assert_eq!(written.get(key), Some(value), "{id}: {key}");
            }
        } else {
            assert_eq!(written, fixture, "{id}");
        }
        let again: CheckConfig = serde_json::from_value(Value::Object(
            written
                .into_iter()
                .chain([("type".to_owned(), Value::String(kind.clone()))])
                .collect(),
        ))
        .unwrap();
        assert_eq!(again, check.config, "{id}");
        assert_ne!(checks::for_device(&pool, device).await.unwrap().len(), 0);
    }
    let web_ui: CheckId = "0190a000-0000-7000-8003-00000000001a".parse().unwrap();
    let CheckConfig::Http(http) = checks::get(&pool, web_ui).await.unwrap().config else {
        panic!("not http")
    };
    assert!(!http.verify_tls);
}
