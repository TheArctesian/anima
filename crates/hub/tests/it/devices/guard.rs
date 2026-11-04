//! Every devices operation needs a signed-in operator (API-003).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{error_json, schema_for};
use async_graphql::Request;

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const HOME_SITE: &str = "0190a000-0000-7000-8001-000000000001";

/// Every root field the devices part declares, with arguments that would succeed if the
/// guard let them through.
fn operations() -> Vec<(&'static str, String)> {
    let device = format!(r#"{{ siteId: "{HOME_SITE}", name: "pve-02", host: "pve-02.home.lan" }}"#);
    vec![
        ("sites", "{ sites { id } }".to_owned()),
        (
            "device",
            format!(r#"{{ device(id: "{PVE_01}") {{ id }} }}"#),
        ),
        ("devices", "{ devices { totalCount } }".to_owned()),
        ("tags", "{ tags }".to_owned()),
        (
            "createSite",
            r#"mutation { createSite(name: "office-3", subnet: "10.40.0.0/24") { id } }"#
                .to_owned(),
        ),
        (
            "createDevice",
            format!("mutation {{ createDevice(input: {device}) {{ id }} }}"),
        ),
        (
            "updateDevice",
            format!(
                r#"mutation {{ updateDevice(id: "{PVE_01}", input: {{ maintenance: true }}) {{ id }} }}"#
            ),
        ),
        (
            "deleteDevice",
            format!(r#"mutation {{ deleteDevice(id: "{PVE_01}") }}"#),
        ),
    ]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("devices"))
)]
async fn api_003_every_devices_operation_without_an_operator_is_unauthenticated_and_changes_nothing(
    pool: PgPool,
) {
    let schema = schema_for(&pool);
    let before = snapshot(&pool).await;
    let operations = operations();
    assert_eq!(operations.len(), 8);
    for (name, query) in operations {
        let response = schema.execute(Request::new(query)).await;
        let error = error_json(&response);
        assert_eq!(
            error["extensions"]["code"], "UNAUTHENTICATED",
            "{name}: {error}"
        );
        assert_eq!(response.data.into_json().unwrap(), json!(null), "{name}");
    }
    assert_eq!(snapshot(&pool).await, before);
}

async fn snapshot(pool: &PgPool) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM devices.sites), (SELECT count(*) FROM devices.devices), \
                (SELECT count(*) FROM devices.device_tags), \
                (SELECT count(*) FROM events.event_log), (SELECT count(*) FROM events.audit_log)",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
