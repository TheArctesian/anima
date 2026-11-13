//! The four check mutations: what they store, audit, record and roll back (CHK-005, API-003,
//! LOG-004, LOG-005, LOG-007).

use anima_hub::fleet::events::{Signal, Source};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CREATE, DOCKER_HOST, PVE_01, UPDATE, code, create, data_with, error_json, input, now,
    schema_for, schema_with_bus, with_variables,
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

pub async fn rows(pool: &PgPool, table: &str) -> Vec<Value> {
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM events.{table} t ORDER BY at, id"
    ))
    .fetch_all(pool)
    .await
    .unwrap()
}

pub fn ssh_port() -> Value {
    json!({ "tcp": { "port": 22 } })
}

/// (`operator_edited`, `repair_tried_at` is set) of the check (CHK-013).
async fn marker(pool: &PgPool, id: &str) -> (bool, bool) {
    sqlx::query_as(
        "SELECT operator_edited, repair_tried_at IS NOT NULL FROM checks.checks WHERE id = $1::uuid",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn audit(pool: &PgPool, target_type: &str, target_id: &str, action: &str) {
    sqlx::query(
        "INSERT INTO events.audit_log \
         (id, at, operator_id, operator_name, action, target_type, target_id, client_addr, change) \
         VALUES (gen_random_uuid(), $1, gen_random_uuid(), 'op', $2, $3, $4, '127.0.0.1', '{}')",
    )
    .bind(now())
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await
    .unwrap();
}

/// The `do $$ ... $$` block of migration 0302, which the test runs against seeded audit rows.
fn backfill_block() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/0302_checks_method_repair.sql"
    );
    let sql = std::fs::read_to_string(path).unwrap();
    sql[sql.find("do $$").unwrap()..].to_owned()
}

const DELETE: &str = "mutation($id: ID!) { deleteCheck(id: $id) }";
const PAUSE: &str = "mutation($id: ID!, $paused: Boolean!) { \
    setCheckPaused(id: $id, paused: $paused) { id paused } }";

fixture_test! {
    async fn chk_005_create_check_returns_each_config_type(pool: PgPool) {
        let schema = schema_for(&pool);
        let configs = [
            ("ICMP", "IcmpConfig", json!({ "icmp": {} })),
            ("TCP", "TcpConfig", ssh_port()),
            ("TLS", "TlsConfig", json!({ "tls": {} })),
            ("DNS", "DnsConfig", json!({ "dns": { "name": "example.com" } })),
            ("HTTP", "HttpConfig", json!({ "http": { "url": "http://web.lan/" } })),
            ("PUSH", "PushConfig", json!({ "push": true })),
            ("GROUP", "GroupConfig", json!({ "group": { "childIds": [] } })),
        ];
        for (kind, typename, config) in configs {
            let vars = json!({ "input": input(PVE_01, kind, config) });
            let created = data_with(&schema, CREATE, vars).await;
            assert_eq!(created["createCheck"]["type"], kind);
            assert_eq!(created["createCheck"]["config"]["__typename"], typename);
        }
    }

    async fn chk_005_create_applies_the_contract_defaults(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let query = "query($id: ID!) { check(id: $id) { intervalSeconds retries \
            retryIntervalSeconds degradedThresholdMs invert paused \
            config { ... on TcpConfig { port timeoutMs } } } }";
        let read = data_with(&schema, query, json!({ "id": id })).await;
        assert_eq!(read["check"], json!({
            "intervalSeconds": 60, "retries": 3, "retryIntervalSeconds": 20,
            "degradedThresholdMs": 1000, "invert": false, "paused": false,
            "config": { "port": 22, "timeoutMs": 5000 },
        }));
    }

    async fn log_005_create_writes_one_audit_entry_and_one_event_in_the_same_second(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let (events, audits) = (rows(&pool, "event_log").await, rows(&pool, "audit_log").await);
        assert_eq!((events.len(), audits.len()), (1, 1));
        let (event, audit) = (&events[0], &audits[0]);
        assert_eq!(event["source"], Source::Check.as_str());
        assert_eq!(event["device_name"], "pve-01");
        assert_eq!(event["message"], "Check ssh on pve-01 added.");
        assert_eq!((audit["action"].clone(), audit["target_type"].clone()), (json!("create"), json!("check")));
        assert_eq!(audit["target_id"], id);
        let at: chrono::DateTime<chrono::Utc> = audit["at"].as_str().unwrap().parse().unwrap();
        assert_eq!(at, now());
        assert_eq!(audit["at"], event["at"]);
        assert_eq!(audit["change"]["name"], "ssh");
        assert_eq!(audit["change"]["config"], json!({ "type": "tcp", "port": 22, "timeoutMs": 5000 }));
        assert!(matches!(receiver.try_recv(), Ok(Signal::Event(_))));
    }

    async fn log_005_update_records_only_the_fields_that_changed(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let vars = json!({ "id": id, "input": { "name": "ssh2", "retries": 5 } });
        data_with(&schema, UPDATE, vars).await;
        let audits = rows(&pool, "audit_log").await;
        assert_eq!(audits.len(), 2);
        assert_eq!(audits[1]["action"], "update");
        assert_eq!(audits[1]["change"], json!({ "name": "ssh2", "retries": 5 }));
        let events = rows(&pool, "event_log").await;
        assert_eq!(events[1]["message"], "Check ssh2 on pve-01 updated.");
    }

    async fn chk_005_update_can_leave_a_group_with_a_null_parent(pool: PgPool) {
        let schema = schema_for(&pool);
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [] } }))).await;
        let mut child = input(PVE_01, "ssh", ssh_port());
        child["parentId"] = json!(group);
        let child = create(&schema, child).await;
        let parent = "query($id: ID!) { check(id: $id) { parent { id } } }";
        let before = data_with(&schema, parent, json!({ "id": child })).await;
        assert_eq!(before["check"]["parent"]["id"], group);
        // Absent parentId keeps the group.
        data_with(&schema, UPDATE, json!({ "id": child, "input": { "retries": 1 } })).await;
        let kept = data_with(&schema, parent, json!({ "id": child })).await;
        assert_eq!(kept["check"]["parent"]["id"], group);
        data_with(&schema, UPDATE, json!({ "id": child, "input": { "parentId": null } })).await;
        let after = data_with(&schema, parent, json!({ "id": child })).await;
        assert_eq!(after["check"]["parent"], Value::Null);
    }

    async fn chk_005_delete_returns_the_id_and_audits_the_name(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let deleted = data_with(&schema, DELETE, json!({ "id": id })).await;
        assert_eq!(deleted["deleteCheck"], id);
        let audits = rows(&pool, "audit_log").await;
        assert_eq!(audits[1]["action"], "delete");
        assert_eq!(audits[1]["change"], json!({ "name": "ssh", "type": "tcp" }));
        let gone = with_variables(&schema, DELETE, json!({ "id": id })).await;
        assert_eq!(code(&error_json(&gone)), "NOT_FOUND");
        assert_eq!(rows(&pool, "audit_log").await.len(), 2);
    }

    async fn chk_005_pausing_and_resuming_audit_pause_and_resume(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let paused = data_with(&schema, PAUSE, json!({ "id": id, "paused": true })).await;
        assert_eq!(paused["setCheckPaused"]["paused"], true);
        data_with(&schema, PAUSE, json!({ "id": id, "paused": false })).await;
        let actions: Vec<Value> = rows(&pool, "audit_log").await.iter().map(|a| a["action"].clone()).collect();
        assert_eq!(actions, [json!("create"), json!("pause"), json!("resume")]);
        let missing = json!({ "id": "0190a000-0000-7000-8003-0000000000ff", "paused": true });
        let error = error_json(&with_variables(&schema, PAUSE, missing).await);
        assert_eq!(code(&error), "NOT_FOUND");
    }

    async fn chk_013_only_an_operator_update_marks_a_check_edited(pool: PgPool) {
        let schema = schema_for(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        assert_eq!(marker(&pool, &id).await, (false, false));
        data_with(&schema, PAUSE, json!({ "id": id, "paused": true })).await;
        data_with(&schema, PAUSE, json!({ "id": id, "paused": false })).await;
        assert_eq!(marker(&pool, &id).await, (false, false));
        data_with(&schema, UPDATE, json!({ "id": id, "input": { "retries": 5 } })).await;
        assert_eq!(marker(&pool, &id).await, (true, false));
    }

    async fn chk_013_a_group_update_does_not_mark_its_children(pool: PgPool) {
        let schema = schema_for(&pool);
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [] } }))).await;
        let mut child = input(PVE_01, "ssh", ssh_port());
        child["parentId"] = json!(group);
        let child = create(&schema, child).await;
        data_with(&schema, UPDATE, json!({ "id": group, "input": { "name": "renamed" } })).await;
        assert_eq!(marker(&pool, &group).await, (true, false));
        assert_eq!(marker(&pool, &child).await, (false, false));
    }

    async fn chk_013_creating_a_group_with_children_does_not_mark_them_edited(pool: PgPool) {
        let schema = schema_for(&pool);
        let child = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [child] } }))).await;
        assert_eq!(marker(&pool, &child).await, (false, false));
        assert_eq!(marker(&pool, &group).await, (false, false));
    }

    async fn chk_013_regrouping_by_a_group_update_marks_the_group_but_not_the_child(pool: PgPool) {
        let schema = schema_for(&pool);
        let child = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [] } }))).await;
        let vars = json!({ "id": group, "input": { "config": { "group": { "childIds": [child] } } } });
        data_with(&schema, UPDATE, vars).await;
        assert_eq!(marker(&pool, &group).await, (true, false));
        assert_eq!(marker(&pool, &child).await, (false, false));
        let vars = json!({ "id": group, "input": { "config": { "group": { "childIds": [] } } } });
        data_with(&schema, UPDATE, vars).await;
        assert_eq!(marker(&pool, &child).await, (false, false));
    }

    async fn chk_013_the_backfill_marks_only_checks_with_an_audited_update(pool: PgPool) {
        let schema = schema_for(&pool);
        let edited = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        let untouched = create(&schema, input(PVE_01, "other", ssh_port())).await;
        sqlx::query("UPDATE checks.checks SET operator_edited = false").execute(&pool).await.unwrap();
        // Another target type with the same id, and a pause entry: neither counts.
        audit(&pool, "device", &untouched, "update").await;
        audit(&pool, "check", &untouched, "pause").await;
        audit(&pool, "check", &edited, "update").await;
        sqlx::raw_sql(&backfill_block()).execute(&pool).await.unwrap();
        assert_eq!(marker(&pool, &edited).await, (true, false));
        assert_eq!(marker(&pool, &untouched).await, (false, false));
    }

    async fn api_003_a_bad_name_or_device_is_a_validation_error_that_stores_nothing(pool: PgPool) {
        let schema = schema_for(&pool);
        let blank = json!({ "input": input(PVE_01, "  ", ssh_port()) });
        let error = error_json(&with_variables(&schema, CREATE, blank).await);
        assert_eq!(super::harness::validation_fields(&error), ["name"]);
        let nowhere = json!({ "input": input("0190a000-0000-7000-8002-0000000000ff", "x", ssh_port()) });
        let error = error_json(&with_variables(&schema, CREATE, nowhere).await);
        assert_eq!(super::harness::validation_fields(&error), ["deviceId"]);
        let wide = json!({ "input": { "deviceId": PVE_01, "name": "x", "config": ssh_port(), "retries": 300 } });
        let error = error_json(&with_variables(&schema, CREATE, wide).await);
        assert_eq!(super::harness::validation_fields(&error), ["retries"]);
        assert_eq!(rows(&pool, "audit_log").await.len(), 0);
    }

    async fn log_005_a_failed_audit_rolls_back_every_check_write_and_shows_no_sql(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "ssh", ssh_port())).await;
        sqlx::query("DROP TABLE events.audit_log").execute(&pool).await.unwrap();
        let mut receiver = bus.subscribe();
        let calls = [
            (CREATE, json!({ "input": input(DOCKER_HOST, "other", ssh_port()) })),
            (UPDATE, json!({ "id": id, "input": { "name": "renamed" } })),
            (PAUSE, json!({ "id": id, "paused": true })),
            (DELETE, json!({ "id": id })),
        ];
        for (query, variables) in calls {
            let error = error_json(&with_variables(&schema, query, variables).await);
            assert_eq!(code(&error), "INTERNAL", "{error}");
            let text = error.to_string();
            for leak in ["audit_log", "SELECT", "INSERT", "relation", "events."] {
                assert!(!text.contains(leak), "{text}");
            }
        }
        let stored: Vec<(String, bool)> = sqlx::query_as("SELECT name, paused FROM checks.checks")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(stored, [("ssh".to_owned(), false)]);
        // The audit row is written first, so no subscriber saw an event that was rolled back.
        assert!(receiver.try_recv().is_err());
    }
}
