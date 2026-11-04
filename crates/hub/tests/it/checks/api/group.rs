//! `GroupConfigInput.childIds` sets the parent of its children in the mutation's own
//! transaction (CHK-004, CHK-005).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    CREATE, DOCKER_HOST, PVE_01, UPDATE, create, data_with, error_json, input, schema_for,
    validation_fields, with_variables,
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
}
