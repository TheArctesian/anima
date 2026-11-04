//! Edges of the mutations: one connection is enough, an empty update is refused, a push check's
//! token rotates only on request, and a deleted group says whom it let go (API-010, CHK-005).

use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};

use super::{
    harness::{
        CREATE, PVE_01, UPDATE, code, create, data_with, error_json, input, now, schema_at,
        schema_with_bus, validation_fields, with_variables,
    },
    writes::rows,
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

const PATH: &str =
    "query($id: ID!) { check(id: $id) { config { ... on PushConfig { pushPath } } } }";

async fn path_of(schema: &super::harness::TestSchema, id: &str) -> String {
    let read = data_with(schema, PATH, json!({ "id": id })).await;
    read["check"]["config"]["pushPath"]
        .as_str()
        .unwrap()
        .to_owned()
}

fixture_test! {
    async fn api_010_every_mutation_works_with_a_single_pool_connection(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let tcp = json!({ "tcp": { "port": 22 } });
        let id = create(&schema, input(PVE_01, "ssh", tcp.clone())).await;
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [] } }))).await;
        let single = PgPoolOptions::new()
            .max_connections(1)
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        let (schema, _) = schema_at(&single, now());
        let calls = [
            (CREATE, json!({ "input": input(PVE_01, "second", tcp) })),
            (UPDATE, json!({ "id": id, "input": { "name": "renamed" } })),
            ("mutation($id: ID!) { setCheckPaused(id: $id, paused: true) { id } }", json!({ "id": id })),
            ("mutation($id: ID!) { deleteCheck(id: $id) }", json!({ "id": group })),
        ];
        for (query, variables) in calls {
            let response = with_variables(&schema, query, variables).await;
            assert_eq!(response.errors.len(), 0, "{query}: {:?}", response.errors);
        }
    }

    async fn api_003_an_update_that_names_nothing_is_refused_and_writes_nothing(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "ssh", json!({ "tcp": { "port": 22 } }))).await;
        let error = error_json(&with_variables(&schema, UPDATE, json!({ "id": id, "input": {} })).await);
        assert_eq!(validation_fields(&error), ["input"]);
        assert_eq!(rows(&pool, "audit_log").await.len(), 1);
        assert_eq!(rows(&pool, "event_log").await.len(), 1);
    }

    async fn chk_005_push_false_is_a_validation_error_on_create_and_update(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let vars = json!({ "input": input(PVE_01, "p", json!({ "push": false })) });
        let error = error_json(&with_variables(&schema, CREATE, vars).await);
        assert_eq!(validation_fields(&error), ["config"]);
        let id = create(&schema, input(PVE_01, "p", json!({ "push": true }))).await;
        let vars = json!({ "id": id, "input": { "config": { "push": false } } });
        let error = error_json(&with_variables(&schema, UPDATE, vars).await);
        assert_eq!(validation_fields(&error), ["config"]);
        assert_eq!(code(&error), "VALIDATION");
    }

    async fn chk_005_the_push_token_changes_only_when_the_update_sends_push_true(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "p", json!({ "push": true }))).await;
        let first = path_of(&schema, &id).await;
        data_with(&schema, UPDATE, json!({ "id": id, "input": { "name": "renamed" } })).await;
        assert_eq!(path_of(&schema, &id).await, first, "a rename keeps the token");
        data_with(&schema, UPDATE, json!({ "id": id, "input": { "config": { "push": true } } })).await;
        let second = path_of(&schema, &id).await;
        assert_ne!(second, first);
        assert!(second.starts_with("/push/"));
        let audits = rows(&pool, "audit_log").await;
        let change = &audits[2]["change"];
        assert_eq!(change["pushPath"], "[changed]");
        let text = serde_json::to_string(&audits).unwrap();
        for token in [&first, &second] {
            assert!(!text.contains(token.trim_start_matches("/push/")));
        }
    }

    async fn chk_005_deleting_a_group_audits_the_children_it_let_go(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let child = create(&schema, input(PVE_01, "ssh", json!({ "tcp": { "port": 22 } }))).await;
        let group = create(&schema, input(PVE_01, "group", json!({ "group": { "childIds": [child] } }))).await;
        data_with(&schema, "mutation($id: ID!) { deleteCheck(id: $id) }", json!({ "id": group })).await;
        let audits = rows(&pool, "audit_log").await;
        let last: &Value = &audits[audits.len() - 1];
        assert_eq!(last["action"], "delete");
        assert_eq!(last["change"]["childIds"], json!([child]));
    }
}
