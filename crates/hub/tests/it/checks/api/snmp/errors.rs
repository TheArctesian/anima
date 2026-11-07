//! SNMP field errors and refused shapes through the API: each names the field and quotes no
//! secret, in the response or the log (D-5, LOG-007).

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{COMMUNITY, SECRETS, v3};
use crate::checks::api::harness::{
    CREATE, PVE_01, UPDATE, assert_absent, captured, create, error_json, hardened_schema, input,
    schema_with_bus, validation_fields, with_variables,
};

macro_rules! fixture_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../fixtures", scripts("devices"))
        )]
        async fn $name($pool: PgPool) $body
    )+};
}

/// Sends the mutation with the log captured; returns the error and the response and log text.
async fn refused(
    schema: &crate::checks::api::harness::TestSchema,
    query: &str,
    vars: Value,
) -> (Value, [String; 3]) {
    let (log, response) = captured(|| with_variables(schema, query, vars)).await;
    assert!(!response.errors.is_empty(), "the input was accepted");
    let texts = [
        serde_json::to_string(&response).unwrap(),
        format!("{:?}", response.errors),
        log,
    ];
    let places: Vec<&str> = texts.iter().map(String::as_str).collect();
    for secret in SECRETS {
        assert_absent(secret, &places);
    }
    (error_json(&response), texts)
}

fn with(edit: impl FnOnce(&mut Value)) -> Value {
    let mut config = v3("monitor");
    edit(&mut config["snmp"]);
    config
}

fixture_test! {
    async fn d_5_field_errors_name_the_field_and_quote_no_secret(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let cases = [
            (with(|c| { c.as_object_mut().unwrap().remove("authPassword"); }), "authPassword"),
            (with(|c| c["privacyPassword"] = json!("short")), "privacyPassword"),
            (with(|c| c["username"] = json!("u".repeat(33))), "username"),
            (with(|c| c["oid"] = json!("1.3.6.x")), "oid"),
            (with(|c| c["port"] = json!(70_000)), "port"),
            (with(|c| c["community"] = json!(COMMUNITY)), "community"),
            (json!({ "snmp": { "oid": "1.3.6.1.2.1.1.5.0" } }), "community"),
        ];
        for (config, field) in cases {
            let vars = json!({ "input": input(PVE_01, "snmp", config) });
            let (error, _) = refused(&schema, CREATE, vars).await;
            assert_eq!(validation_fields(&error), [field]);
        }
    }

    async fn sec_005_a_changed_user_name_without_passwords_is_a_field_error(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "snmp", v3("monitor"))).await;
        let renamed = json!({ "snmp": { "version": "V3", "oid": "1.3.6.1.2.1.1.5.0", "username": "other" } });
        let vars = json!({ "id": id, "input": { "config": renamed } });
        let (error, _) = refused(&schema, UPDATE, vars).await;
        assert_eq!(validation_fields(&error), ["authPassword"]);
    }

    async fn log_007_an_snmp_input_that_does_not_parse_is_refused_without_quoting_it(pool: PgPool) {
        let schema = hardened_schema(&pool);
        let malformed = [
            json!({ "snmp": { "oid": 5, "community": COMMUNITY } }),
            json!({ "snmp": { "oid": "1.3", "community": [COMMUNITY] } }),
            json!({ "snmp": { "oid": "1.3", "authPassword": { "inner": super::AUTH } } }),
            json!({ "snmp": { "oid": "1.3", "version": "V9", "privacyPassword": super::PRIVACY } }),
            json!({ "snmp": { "oid": "1.3", "community": COMMUNITY, "extra": COMMUNITY } }),
            json!({ "snmp": { "oid": "1.3", "community": COMMUNITY }, "tcp": { "port": 1 } }),
        ];
        for config in malformed {
            let vars = json!({ "input": input(PVE_01, "snmp", config) });
            let (error, _) = refused(&schema, CREATE, vars).await;
            let code = error["extensions"]["code"].as_str();
            assert!(matches!(code, None | Some("VALIDATION")), "{error}");
        }
        let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.checks WHERE type = 'snmp'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, 0);
    }
}
