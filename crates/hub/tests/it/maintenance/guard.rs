//! Every maintenance operation needs a signed-in operator (API-003).

use async_graphql::Request;
use sqlx::PgPool;

use super::harness::{error_json, schema_for};

const SUNDAY_UPDATES: &str = "0190a000-0000-7000-8004-000000000001";
const FREEZE: &str = "0190a000-0000-7000-8004-0000000000a1";

/// Every root field the maintenance part declares, with arguments that would succeed if the
/// guard let them through.
fn operations() -> Vec<(&'static str, String)> {
    let input = r#"{ name: "Patch", kind: MANUAL, timezone: "UTC", targets: [{ type: TAG, target: "infra" }] }"#;
    vec![
        (
            "maintenanceWindows",
            "{ maintenanceWindows { id } }".to_owned(),
        ),
        (
            "createMaintenanceWindow",
            format!("mutation {{ createMaintenanceWindow(input: {input}) {{ id }} }}"),
        ),
        (
            "updateMaintenanceWindow",
            format!(
                r#"mutation {{ updateMaintenanceWindow(id: "{FREEZE}", input: {input}) {{ id }} }}"#
            ),
        ),
        (
            "deleteMaintenanceWindow",
            format!(r#"mutation {{ deleteMaintenanceWindow(id: "{SUNDAY_UPDATES}") }}"#),
        ),
        (
            "setManualWindowActive",
            format!(
                r#"mutation {{ setManualWindowActive(id: "{FREEZE}", active: true) {{ id }} }}"#
            ),
        ),
    ]
}

async fn snapshot(pool: &PgPool) -> Vec<(String, String, bool, i64)> {
    sqlx::query_as(
        "SELECT w.name, w.rule::text, w.active_manual, \
                (SELECT count(*) FROM maintenance.window_targets t WHERE t.window_id = w.id) \
         FROM maintenance.windows w ORDER BY w.name",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../fixtures", scripts("maintenance"))
)]
async fn api_003_every_maintenance_operation_without_an_operator_is_unauthenticated_and_changes_nothing(
    pool: PgPool,
) {
    sqlx::query(
        "INSERT INTO maintenance.windows (id, name, kind, timezone, rule) \
         VALUES ($1::text::uuid, 'Freeze', 'manual', 'UTC', '{\"kind\": \"manual\"}')",
    )
    .bind(FREEZE)
    .execute(&pool)
    .await
    .unwrap();
    let schema = schema_for(&pool);
    let before = snapshot(&pool).await;
    let operations = operations();
    assert_eq!(operations.len(), 5);
    for (name, query) in operations {
        let response = schema.execute(Request::new(query)).await;
        let error = error_json(&response);
        assert_eq!(
            error["extensions"]["code"], "UNAUTHENTICATED",
            "{name}: {error}"
        );
        assert_eq!(snapshot(&pool).await, before, "{name} changed the windows");
    }
    let logged: i64 = sqlx::query_scalar("SELECT count(*) FROM events.audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(logged, 0);
}
