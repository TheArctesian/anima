//! `GroupConfigInput.childIds` sets the parent of its children in the mutation's own
//! transaction (CHK-004, CHK-005).

use anima_hub::status::Status;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CREATE, DOCKER_HOST, PVE_01, UPDATE, beat, create, data_with, database_now, error_json, input,
    record, schema_at, schema_for, validation_fields, with_variables,
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

const CHILDREN: &str = "query($id: ID!) { check(id: $id) { config { ... on GroupConfig { \
    children { name } } } } }";

fn group_of(ids: &[&str]) -> Value {
    json!({ "group": { "childIds": ids } })
}

async fn tcp(schema: &super::harness::TestSchema, device: &str, name: &str) -> String {
    create(
        schema,
        input(device, name, json!({ "tcp": { "port": 22 } })),
    )
    .await
}

async fn names(schema: &super::harness::TestSchema, group: &str) -> Vec<String> {
    let read = data_with(schema, CHILDREN, json!({ "id": group })).await;
    read["check"]["config"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect()
}

fixture_test! {
    async fn chk_004_a_group_created_with_child_ids_adopts_those_children(pool: PgPool) {
        let schema = schema_for(&pool);
        let (a, b) = (tcp(&schema, PVE_01, "a").await, tcp(&schema, PVE_01, "b").await);
        tcp(&schema, PVE_01, "outside").await;
        let group = create(&schema, input(PVE_01, "group", group_of(&[&b, &a]))).await;
        assert_eq!(names(&schema, &group).await, ["a", "b"]);
        let audit: Value = sqlx::query_scalar(
            "SELECT change FROM events.audit_log WHERE change->'childIds' IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit["childIds"], json!([b, a]));
    }

    async fn chk_004_updating_the_group_replaces_its_children(pool: PgPool) {
        let schema = schema_for(&pool);
        let (a, b, c) = (
            tcp(&schema, PVE_01, "a").await,
            tcp(&schema, PVE_01, "b").await,
            tcp(&schema, PVE_01, "c").await,
        );
        let group = create(&schema, input(PVE_01, "group", group_of(&[&a, &b]))).await;
        let vars = json!({ "id": group, "input": { "config": group_of(&[&b, &c]) } });
        data_with(&schema, UPDATE, vars).await;
        assert_eq!(names(&schema, &group).await, ["b", "c"]);
    }

    async fn chk_004_a_child_on_another_device_is_refused_and_nothing_is_stored(pool: PgPool) {
        let schema = schema_for(&pool);
        let far = tcp(&schema, DOCKER_HOST, "far").await;
        let vars = json!({ "input": input(PVE_01, "group", group_of(&[&far])) });
        let error = error_json(&with_variables(&schema, CREATE, vars).await);
        assert_eq!(validation_fields(&error), ["childIds"]);
        let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.checks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, 1);
    }

    async fn chk_004_a_group_cannot_be_a_child_and_a_missing_child_is_refused(pool: PgPool) {
        let schema = schema_for(&pool);
        let inner = create(&schema, input(PVE_01, "inner", group_of(&[]))).await;
        for child in [inner.as_str(), "0190a000-0000-7000-8003-0000000000ff"] {
            let vars = json!({ "input": input(PVE_01, "outer", group_of(&[child])) });
            let error = error_json(&with_variables(&schema, CREATE, vars).await);
            assert_eq!(validation_fields(&error), ["childIds"]);
        }
        let bad_id = json!({ "input": input(PVE_01, "outer", group_of(&["not-an-id"])) });
        let error = error_json(&with_variables(&schema, CREATE, bad_id).await);
        assert_eq!(validation_fields(&error), ["childIds"]);
    }

    async fn chk_004_a_bad_update_of_the_children_changes_no_check(pool: PgPool) {
        let schema = schema_for(&pool);
        let a = tcp(&schema, PVE_01, "a").await;
        let group = create(&schema, input(PVE_01, "group", group_of(&[&a]))).await;
        let far = tcp(&schema, DOCKER_HOST, "far").await;
        let vars = json!({ "id": group, "input": { "name": "renamed", "config": group_of(&[&far]) } });
        let error = error_json(&with_variables(&schema, UPDATE, vars).await);
        assert_eq!(validation_fields(&error), ["childIds"]);
        assert_eq!(names(&schema, &group).await, ["a"]);
        let name: String = sqlx::query_scalar("SELECT name FROM checks.checks WHERE id = $1::text::uuid")
            .bind(&group)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "group");
    }

    async fn chk_004_a_group_shows_the_worst_status_of_the_children_that_are_not_paused(pool: PgPool) {
        let at = database_now(&pool).await;
        let schema = schema_at(&pool, at).0;
        let (good, bad) = (tcp(&schema, PVE_01, "good").await, tcp(&schema, PVE_01, "bad").await);
        let group = create(&schema, input(PVE_01, "group", group_of(&[&good, &bad]))).await;
        record(&pool, &good, &beat(at, Status::Up, Some(1))).await;
        record(&pool, &bad, &beat(at, Status::Down, None)).await;
        let status = "query($id: ID!) { check(id: $id) { status } }";
        let shown = data_with(&schema, status, json!({ "id": group })).await;
        assert_eq!(shown["check"]["status"], "DOWN");
        let pause = "mutation($id: ID!) { setCheckPaused(id: $id, paused: true) { id } }";
        data_with(&schema, pause, json!({ "id": bad })).await;
        let shown = data_with(&schema, status, json!({ "id": group })).await;
        assert_eq!(shown["check"]["status"], "UP");
    }

    async fn mw_001_a_window_over_the_device_turns_down_into_maintenance_for_check_and_group(pool: PgPool) {
        let at = database_now(&pool).await;
        let schema = schema_at(&pool, at).0;
        let bad = tcp(&schema, PVE_01, "bad").await;
        let group = create(&schema, input(PVE_01, "group", group_of(&[&bad]))).await;
        record(&pool, &bad, &beat(at, Status::Down, None)).await;
        let status = "query($id: ID!) { check(id: $id) { status } }";
        for id in [&bad, &group] {
            let read = data_with(&schema, status, json!({ "id": id })).await;
            assert_eq!(read["check"]["status"], "DOWN");
        }
        sqlx::query(
            "INSERT INTO maintenance.windows (id, name, kind, timezone, rule, active_manual) \
             VALUES ('0190a000-0000-7000-8004-0000000000aa', 'window', 'manual', 'UTC', \
                     '{\"kind\": \"manual\"}', true)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO maintenance.window_targets (window_id, target_type, target) \
             VALUES ('0190a000-0000-7000-8004-0000000000aa', 'device', $1)",
        )
        .bind(PVE_01)
        .execute(&pool)
        .await
        .unwrap();
        for id in [&bad, &group] {
            let read = data_with(&schema, status, json!({ "id": id })).await;
            assert_eq!(read["check"]["status"], "MAINTENANCE");
        }
    }
}
