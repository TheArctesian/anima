//! The four window mutations: what they store, return and refuse (MW-001 to MW-004, API-003).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    DELETE, create, create_query, data_with, error_json, manual_input, schema_for, switch_query,
    update_query, validation_fields, weekday_input, with_variables,
};

const SUNDAY_UPDATES: &str = "0190a000-0000-7000-8004-000000000001";
const MISSING: &str = "0190a000-0000-7000-8004-0000000000ff";

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../fixtures", scripts("maintenance"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

async fn window_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM maintenance.windows")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn target_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM maintenance.window_targets")
        .fetch_one(pool)
        .await
        .unwrap()
}

fixture_test! {
    async fn mw_004_create_returns_the_stored_window_with_the_fields_as_typed(pool: PgPool) {
        let created = data_with(&schema_for(&pool), &create_query(), json!({ "input": weekday_input("Patch") })).await;
        let window = &created["createMaintenanceWindow"];
        assert_eq!(window["name"], "Patch");
        assert_eq!(window["kind"], "RECURRING_WEEKDAY");
        assert_eq!(window["timezone"], "Pacific/Auckland");
        assert_eq!(window["weekdays"], json!(["SUN"]));
        assert_eq!(window["startTime"], "03:00");
        assert_eq!(window["durationMinutes"], 60);
        assert_eq!(window["targets"], json!([{ "type": "TAG", "target": "infra" }]));
        assert_eq!(window["activeManual"], Value::Null);
        assert_eq!(window["startsAt"], Value::Null);
        assert_eq!(window_count(&pool).await, 3);
    }

    async fn mw_003_create_stores_every_target_and_trims_and_deduplicates_tags(pool: PgPool) {
        let mut input = manual_input("Many targets");
        input["targets"] = json!([
            { "type": "TAG", "target": "  infra " },
            { "type": "TAG", "target": "infra" },
            { "type": "DEVICE", "target": "0190A000-0000-7000-8002-000000000001" },
        ]);
        let created = data_with(&schema_for(&pool), &create_query(), json!({ "input": input })).await;
        assert_eq!(created["createMaintenanceWindow"]["targets"], json!([
            { "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" },
            { "type": "TAG", "target": "infra" },
        ]));
        assert_eq!(target_count(&pool).await, 4);
    }

    async fn mw_001_a_bad_time_zone_is_a_validation_error_and_stores_nothing(pool: PgPool) {
        let mut input = weekday_input("Patch");
        input["timezone"] = json!("Mars/Olympus_Mons");
        let response = with_variables(&schema_for(&pool), &create_query(), json!({ "input": input })).await;
        assert_eq!(validation_fields(&error_json(&response)), ["timezone"]);
        assert_eq!((window_count(&pool).await, target_count(&pool).await), (2, 2));
    }

    async fn mw_004_validation_lists_every_field_in_error_together(pool: PgPool) {
        let input = json!({
            "name": "", "kind": "RECURRING_WEEKDAY", "timezone": "nowhere",
            "weekdays": [], "startTime": "7pm", "durationMinutes": 0, "everyDays": 2,
            "targets": [],
        });
        let response = with_variables(&schema_for(&pool), &create_query(), json!({ "input": input })).await;
        let error = error_json(&response);
        assert_eq!(
            validation_fields(&error),
            ["name", "timezone", "everyDays", "weekdays", "startTime", "durationMinutes", "targets"]
        );
        for field in error["extensions"]["fields"].as_array().unwrap() {
            assert_ne!(field["message"].as_str().unwrap(), "");
        }
        assert_eq!(window_count(&pool).await, 2);
    }

    async fn mw_004_a_field_of_another_kind_is_refused_by_name(pool: PgPool) {
        let mut input = manual_input("Manual with a rule");
        input["startTime"] = json!("03:00");
        let response = with_variables(&schema_for(&pool), &create_query(), json!({ "input": input })).await;
        assert_eq!(validation_fields(&error_json(&response)), ["startTime"]);
    }

    async fn mw_001_a_name_already_in_use_is_a_validation_error_on_name(pool: PgPool) {
        let response = with_variables(
            &schema_for(&pool), &create_query(), json!({ "input": weekday_input("Sunday updates") }),
        ).await;
        assert_eq!(validation_fields(&error_json(&response)), ["name"]);
        assert_eq!(window_count(&pool).await, 2);
    }

    async fn mw_003_update_replaces_the_rule_and_the_targets(pool: PgPool) {
        let mut input = weekday_input("Sunday updates");
        input["weekdays"] = json!(["SAT", "SUN"]);
        input["durationMinutes"] = json!(120);
        input["targets"] = json!([{ "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" }]);
        let updated = data_with(
            &schema_for(&pool), &update_query(), json!({ "id": SUNDAY_UPDATES, "input": input }),
        ).await;
        let window = &updated["updateMaintenanceWindow"];
        assert_eq!(window["id"], SUNDAY_UPDATES);
        assert_eq!(window["weekdays"], json!(["SAT", "SUN"]));
        assert_eq!(window["durationMinutes"], 120);
        assert_eq!(window["targets"], json!([{ "type": "DEVICE", "target": "0190a000-0000-7000-8002-000000000001" }]));
        // The other window and its target are untouched.
        assert_eq!((window_count(&pool).await, target_count(&pool).await), (2, 2));
    }

    async fn mw_004_update_can_change_the_kind_and_drops_the_old_rule_fields(pool: PgPool) {
        let input = json!({
            "name": "Sunday updates", "kind": "SINGLE", "timezone": "UTC",
            "startsAt": "2026-11-04T09:00:00Z", "endsAt": "2026-11-04T10:00:00Z",
            "targets": [{ "type": "TAG", "target": "infra" }],
        });
        let updated = data_with(
            &schema_for(&pool), &update_query(), json!({ "id": SUNDAY_UPDATES, "input": input }),
        ).await;
        let window = &updated["updateMaintenanceWindow"];
        assert_eq!(window["kind"], "SINGLE");
        assert_eq!(window["weekdays"], Value::Null);
        assert_eq!(window["startTime"], Value::Null);
        assert_eq!(window["durationMinutes"], Value::Null);
        let stored: String = sqlx::query_scalar("SELECT kind FROM maintenance.windows WHERE id = $1::text::uuid")
            .bind(SUNDAY_UPDATES).fetch_one(&pool).await.unwrap();
        assert_eq!(stored, "single");
    }

    async fn mw_001_update_to_a_name_taken_by_another_window_is_a_validation_error(pool: PgPool) {
        let vars = json!({ "id": SUNDAY_UPDATES, "input": weekday_input("Overnight backup") });
        let error = error_json(&with_variables(&schema_for(&pool), &update_query(), vars).await);
        assert_eq!(validation_fields(&error), ["name"]);
        let name: String = sqlx::query_scalar("SELECT name FROM maintenance.windows WHERE id = $1::text::uuid")
            .bind(SUNDAY_UPDATES).fetch_one(&pool).await.unwrap();
        assert_eq!(name, "Sunday updates");
    }

    async fn mw_001_update_may_keep_the_name_of_the_window_it_replaces(pool: PgPool) {
        let vars = json!({ "id": SUNDAY_UPDATES, "input": weekday_input("Sunday updates") });
        let updated = data_with(&schema_for(&pool), &update_query(), vars).await;
        assert_eq!(updated["updateMaintenanceWindow"]["name"], "Sunday updates");
    }

    async fn api_010_update_of_a_missing_window_is_not_found_and_stores_nothing(pool: PgPool) {
        let vars = json!({ "id": MISSING, "input": weekday_input("Ghost") });
        let error = error_json(&with_variables(&schema_for(&pool), &update_query(), vars).await);
        assert_eq!(error["extensions"]["code"], "NOT_FOUND");
        assert_eq!((window_count(&pool).await, target_count(&pool).await), (2, 2));
    }

    async fn api_010_update_reports_a_bad_id_with_the_form_errors(pool: PgPool) {
        let mut input = weekday_input("Patch");
        input["timezone"] = json!("nowhere");
        let vars = json!({ "id": "not-an-id", "input": input });
        let error = error_json(&with_variables(&schema_for(&pool), &update_query(), vars).await);
        assert_eq!(validation_fields(&error), ["id", "timezone"]);
    }

    async fn mw_003_delete_returns_the_id_and_removes_the_window_and_its_targets(pool: PgPool) {
        let deleted = data_with(&schema_for(&pool), DELETE, json!({ "id": SUNDAY_UPDATES })).await;
        assert_eq!(deleted["deleteMaintenanceWindow"], SUNDAY_UPDATES);
        assert_eq!((window_count(&pool).await, target_count(&pool).await), (1, 1));
    }

    async fn api_010_delete_of_a_missing_window_is_not_found(pool: PgPool) {
        let schema = schema_for(&pool);
        let error = error_json(&with_variables(&schema, DELETE, json!({ "id": MISSING })).await);
        assert_eq!(error["extensions"]["code"], "NOT_FOUND");
        let error = error_json(&with_variables(&schema, DELETE, json!({ "id": "x" })).await);
        assert_eq!(validation_fields(&error), ["id"]);
        assert_eq!(window_count(&pool).await, 2);
    }

    async fn mw_004_a_manual_window_is_switched_on_and_off(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, manual_input("Freeze")).await;
        let set = |active: bool| json!({ "id": id, "active": active });

        let on = data_with(&schema, &switch_query(), set(true)).await;
        assert_eq!(on["setManualWindowActive"]["activeManual"], true);
        assert_eq!(on["setManualWindowActive"]["activeNow"], true);
        let off = data_with(&schema, &switch_query(), set(false)).await;
        assert_eq!(off["setManualWindowActive"]["activeManual"], false);
        assert_eq!(off["setManualWindowActive"]["activeNow"], false);
    }

    async fn mw_004_only_a_manual_window_can_be_switched(pool: PgPool) {
        let vars = json!({ "id": SUNDAY_UPDATES, "active": true });
        let error = error_json(&with_variables(&schema_for(&pool), &switch_query(), vars).await);
        assert_eq!(validation_fields(&error), ["id"]);
        let on: i64 = sqlx::query_scalar("SELECT count(*) FROM maintenance.windows WHERE active_manual")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(on, 0);
    }

    async fn api_010_switching_a_missing_window_is_not_found(pool: PgPool) {
        let vars = json!({ "id": MISSING, "active": true });
        let error = error_json(&with_variables(&schema_for(&pool), &switch_query(), vars).await);
        assert_eq!(error["extensions"]["code"], "NOT_FOUND");
    }

    async fn mw_004_replacing_a_manual_window_keeps_its_switch_while_it_stays_manual(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, manual_input("Freeze")).await;
        data_with(&schema, &switch_query(), json!({ "id": id, "active": true })).await;

        let renamed = data_with(
            &schema, &update_query(), json!({ "id": id, "input": manual_input("Freeze 2") }),
        ).await;
        assert_eq!(renamed["updateMaintenanceWindow"]["activeManual"], true);

        let single = json!({
            "name": "Freeze 2", "kind": "SINGLE", "timezone": "UTC",
            "startsAt": "2026-11-04T09:00:00Z", "endsAt": "2026-11-04T10:00:00Z",
            "targets": [{ "type": "TAG", "target": "infra" }],
        });
        data_with(&schema, &update_query(), json!({ "id": id, "input": single })).await;
        let back = data_with(
            &schema, &update_query(), json!({ "id": id, "input": manual_input("Freeze 2") }),
        ).await;
        assert_eq!(back["updateMaintenanceWindow"]["activeManual"], false);
    }
}
