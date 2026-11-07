//! Docker and systemd checks through the API: create, read, update and the field errors
//! (CHK-009, CHK-012, K1 `DockerConfig` and `SystemdConfig`).

use serde_json::json;
use sqlx::PgPool;

use super::harness::{
    CREATE, DOCKER_HOST, UPDATE, create, data_with, error_json, input, schema_with_bus,
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

const READ: &str = "query($id: ID!) { check(id: $id) { type config { __typename \
    ... on DockerConfig { container timeoutMs } ... on SystemdConfig { unit timeoutMs } } } }";

fixture_test! {
    async fn chk_009_a_docker_check_is_created_with_the_contract_default_and_read_back(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let config = json!({ "docker": { "container": "web-1" } });
        let id = create(&schema, input(DOCKER_HOST, "web", config)).await;

        let read = data_with(&schema, READ, json!({ "id": id })).await;

        assert_eq!(read["check"]["type"], "DOCKER");
        assert_eq!(
            read["check"]["config"],
            json!({ "__typename": "DockerConfig", "container": "web-1", "timeoutMs": 10000 })
        );
        let (kind, stored): (String, serde_json::Value) =
            sqlx::query_as("SELECT type, config FROM checks.checks WHERE id = $1::uuid")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "docker");
        assert_eq!(stored, json!({ "container": "web-1", "timeoutMs": 10000 }));
    }

    async fn chk_009_a_systemd_check_is_created_with_a_timeout_and_updated(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let config = json!({ "systemd": { "unit": "nginx", "timeoutMs": 4000 } });
        let id = create(&schema, input(DOCKER_HOST, "nginx", config)).await;
        let read = data_with(&schema, READ, json!({ "id": id })).await;
        assert_eq!(read["check"]["type"], "SYSTEMD");
        assert_eq!(
            read["check"]["config"],
            json!({ "__typename": "SystemdConfig", "unit": "nginx", "timeoutMs": 4000 })
        );

        let changed = json!({ "config": { "systemd": { "unit": "postgresql@16-main.service" } } });
        data_with(&schema, UPDATE, json!({ "id": id, "input": changed })).await;

        let read = data_with(&schema, READ, json!({ "id": id })).await;
        assert_eq!(
            read["check"]["config"],
            json!({
                "__typename": "SystemdConfig",
                "unit": "postgresql@16-main.service",
                "timeoutMs": 10000
            })
        );
    }

    async fn chk_012_a_container_or_unit_name_the_rules_refuse_is_a_field_error_that_quotes_nothing(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let cases = [
            (json!({ "docker": { "container": "web;rm -rf" } }), "container"),
            (json!({ "docker": { "container": "-web" } }), "container"),
            (json!({ "docker": { "container": "" } }), "container"),
            (json!({ "docker": { "container": "web", "timeoutMs": 99 } }), "timeoutMs"),
            (json!({ "systemd": { "unit": "--now" } }), "unit"),
            (json!({ "systemd": { "unit": "nginx.service; id" } }), "unit"),
            (json!({ "systemd": { "unit": "nginx", "timeoutMs": 60001 } }), "timeoutMs"),
            (json!({ "systemd": { "unit": "a".repeat(256) } }), "unit"),
            (json!({ "docker": { "container": "web", "timeoutMs": 3_000_000_000_i64 } }), "input"),
        ];
        for (config, field) in cases {
            let vars = json!({ "input": input(DOCKER_HOST, "x", config.clone()) });
            let response = with_variables(&schema, CREATE, vars).await;
            let error = error_json(&response);
            assert_eq!(validation_fields(&error), [field], "{config}");
            let text = error.to_string();
            for quoted in ["rm -rf", "--now", "id\"", "web;"] {
                assert!(!text.contains(quoted), "{quoted} in {text}");
            }
        }
        let stored: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM checks.checks WHERE type IN ('docker', 'systemd')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, 0);
    }
}
