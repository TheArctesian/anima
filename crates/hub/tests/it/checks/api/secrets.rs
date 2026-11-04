//! No HTTP secret is returned, audited, recorded, broadcast or logged (SEC-003, LOG-007,
//! K1 `HttpHeader.value`).

use std::fmt::Write as _;

use anima_hub::{checks, events::Signal};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    harness::{
        CREATE, PVE_01, UPDATE, assert_absent, captured, create, data_with, error_json,
        hardened_schema, input, schema_with_bus, signed_in, validation_fields, vault,
        with_variables,
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

const PASSWORD: &str = "hunter2-basic-password";
const BEARER: &str = "hunter2-bearer-token";
const AUTHORIZATION: &str = "hunter2-authorization-header";
const PROXY: &str = "hunter2-proxy-header";
const COOKIE: &str = "hunter2-cookie-header";
const SECRETS: [&str; 5] = [PASSWORD, BEARER, AUTHORIZATION, PROXY, COOKIE];

fn http(auth: Value) -> Value {
    let mut config = json!({ "http": {
        "url": "https://web.lan/health",
        "headers": [
            { "name": "Authorization", "value": AUTHORIZATION },
            { "name": "proxy-authorization", "value": PROXY },
            { "name": "COOKIE", "value": COOKIE },
            { "name": "X-Trace", "value": "trace-1" },
        ],
    } });
    config["http"]["auth"] = auth;
    config
}

fn basic() -> Value {
    json!({ "basic": { "username": "bob", "password": PASSWORD } })
}

const READ: &str = "query($id: ID!) { check(id: $id) { config { ... on HttpConfig { \
    authKind headers { name value } url } } } }";

fixture_test! {
    async fn sec_003_the_value_of_a_secret_header_is_null_and_the_auth_kind_is_only_a_kind(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "web", http(basic()))).await;
        let read = data_with(&schema, READ, json!({ "id": id })).await;
        let config = &read["check"]["config"];
        assert_eq!(config["authKind"], "BASIC");
        assert_eq!(config["headers"], json!([
            { "name": "Authorization", "value": null },
            { "name": "proxy-authorization", "value": null },
            { "name": "COOKIE", "value": null },
            { "name": "X-Trace", "value": "trace-1" },
        ]));
        let bearer = create(&schema, input(PVE_01, "bearer", http(json!({ "bearerToken": BEARER })))).await;
        let read = data_with(&schema, READ, json!({ "id": bearer })).await;
        assert_eq!(read["check"]["config"]["authKind"], "BEARER");
        let none = create(&schema, input(PVE_01, "open", json!({ "http": { "url": "http://web.lan/" } }))).await;
        let read = data_with(&schema, READ, json!({ "id": none })).await;
        assert_eq!(read["check"]["config"]["authKind"], "NONE");
    }

    async fn log_007_no_secret_reaches_a_response_row_event_broadcast_or_log(pool: PgPool) {
        let (schema, bus) = schema_with_bus(&pool);
        let mut receiver = bus.subscribe();
        let vars = json!({ "input": input(PVE_01, "web", http(basic())) });
        let (log, created) = captured(|| schema.execute(signed_in(CREATE).variables(
            async_graphql::Variables::from_json(vars),
        ))).await;
        assert_eq!(created.errors.len(), 0);
        let id = created.data.clone().into_json().unwrap()["createCheck"]["id"].as_str().unwrap().to_owned();
        let update = json!({ "id": id, "input": { "name": "web2", "config": http(json!({ "bearerToken": BEARER })) } });
        let updated = with_variables(&schema, UPDATE, update).await;
        assert!(updated.errors.is_empty(), "{:?}", updated.errors);
        let read = data_with(&schema, READ, json!({ "id": id })).await;

        let mut broadcast = String::new();
        while let Ok(signal) = receiver.try_recv() {
            if let Signal::Event(event) = signal {
                write!(broadcast, "{event:?}").unwrap();
            }
        }
        let tables = [
            serde_json::to_string(&rows(&pool, "audit_log").await).unwrap(),
            serde_json::to_string(&rows(&pool, "event_log").await).unwrap(),
        ];
        let config: Value = sqlx::query_scalar("SELECT config FROM checks.checks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let places = [
            serde_json::to_string(&created).unwrap(),
            serde_json::to_string(&updated).unwrap(),
            read.to_string(),
            tables[0].clone(),
            tables[1].clone(),
            config.to_string(),
            broadcast,
            log,
        ];
        let places: Vec<&str> = places.iter().map(String::as_str).collect();
        for secret in SECRETS {
            assert_absent(secret, &places);
        }
        let audit = &rows(&pool, "audit_log").await[0];
        assert_eq!(audit["change"]["sealed"], "[set]");
        assert_eq!(audit["change"]["config"]["headerNames"][0], "Authorization");
    }

    async fn sec_003_secrets_are_sealed_at_rest_and_open_for_the_probe(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "web", http(basic()))).await;
        let sealed: Vec<u8> = sqlx::query_scalar("SELECT secrets_sealed FROM checks.checks")
            .fetch_one(&pool)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&sealed);
        for secret in SECRETS {
            assert!(!text.contains(secret));
        }
        let opened = checks::load_http_secrets(&pool, &vault(), id.parse().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(opened.password.unwrap().expose(), PASSWORD.as_bytes());
        assert_eq!(opened.headers.len(), 3);
    }

    async fn sec_003_an_empty_secret_in_an_update_keeps_the_stored_one(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let id = create(&schema, input(PVE_01, "web", http(basic()))).await;
        let blank = json!({ "http": {
            "url": "https://web.lan/health",
            "headers": [{ "name": "Authorization", "value": "" }],
            "auth": { "basic": { "username": "bob", "password": "" } },
        } });
        let vars = json!({ "id": id, "input": { "config": blank } });
        let updated = with_variables(&schema, UPDATE, vars).await;
        assert!(updated.errors.is_empty(), "{:?}", updated.errors);
        let opened = checks::load_http_secrets(&pool, &vault(), id.parse().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(opened.password.unwrap().expose(), PASSWORD.as_bytes());
        assert_eq!(opened.headers[0].expose(), AUTHORIZATION.as_bytes());
        // No secret was sent, so the audit entry records none.
        let audits = rows(&pool, "audit_log").await;
        assert!(audits[1]["change"].get("sealed").is_none(), "{}", audits[1]["change"]);
    }

    async fn sec_003_an_empty_secret_with_nothing_stored_is_a_validation_error(pool: PgPool) {
        let (schema, _) = schema_with_bus(&pool);
        let blank = json!({ "input": input(PVE_01, "web", http(json!({ "basic": { "username": "bob", "password": "" } }))) });
        let error = error_json(&with_variables(&schema, CREATE, blank).await);
        assert_eq!(validation_fields(&error), ["auth"]);
    }
}

/// Inputs of the wrong shape, each carrying the secret `hunter2-outer`.
fn malformed() -> Vec<Value> {
    let secret = "hunter2-outer";
    let device = PVE_01;
    vec![
        json!(secret),
        json!({ "deviceId": device, "name": "x", "password": secret }),
        json!({ "deviceId": device, "name": 7, "config": { "http": { "url": "http://a/",
            "auth": { "basic": { "username": "u", "password": secret } } } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": "http://a/",
            "auth": { "basic": { "username": 42, "password": secret } } } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": "http://a/",
            "auth": { "basic": { "username": "u", "password": secret }, "bearerToken": secret } } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": "http://a/",
            "auth": { "basic": { "username": "u", "password": [secret] } } } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": "http://a/",
            "headers": [{ "name": "Authorization", "value": { "inner": secret } }] } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": "http://a/",
            "headers": [{ "name": "Authorization", "value": secret, "extra": secret }] } } }),
        json!({ "deviceId": device, "name": "x", "config": { "http": { "url": 5, "bearer": secret } } }),
        json!({ "deviceId": device, "name": "x", "config": { "tcp": { "port": 22 },
            "http": { "url": "http://a/", "auth": { "bearerToken": secret } } } }),
        json!({ "deviceId": device, "name": "x", "config": [secret] }),
    ]
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_007_a_check_input_that_does_not_parse_is_refused_without_quoting_it(pool: PgPool) {
    let schema = hardened_schema(&pool);
    let secret = "hunter2-outer";
    let mut reached_the_parser = 0;
    for (index, bad) in malformed().into_iter().enumerate() {
        let request = || with_variables(&schema, CREATE, json!({ "input": bad }));
        let (log, response) = captured(request).await;
        assert!(!response.errors.is_empty(), "input {index} was accepted");
        let body = serde_json::to_string(&response).unwrap();
        let debug = format!("{:?}", response.errors);
        assert_absent(secret, &[&body, &debug, &log]);
        // Either the input reached the check parser, which answers VALIDATION, or GraphQL
        // validation refused its shape first with a message that names types, not values.
        let error = error_json(&response);
        let refused = error["extensions"]["code"].as_str();
        assert!(
            matches!(refused, None | Some("VALIDATION")),
            "input {index}: {error}"
        );
        reached_the_parser += usize::from(refused.is_some());
    }
    assert!(
        reached_the_parser >= 2,
        "{reached_the_parser} inputs reached the parser"
    );
    let update = "mutation($input: UpdateCheckInput!) { updateCheck(id: \"0190a000-0000-7000-8003-000000000001\", input: $input) { id } }";
    let bad = json!({ "config": { "http": { "url": "http://a/", "auth": { "basic": { "username": 1, "password": secret } } } } });
    let (log, response) =
        captured(|| with_variables(&schema, update, json!({ "input": bad }))).await;
    let body = serde_json::to_string(&response).unwrap();
    assert_absent(secret, &[&body, &format!("{:?}", response.errors), &log]);
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM checks.checks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}

/// Inline literals that GraphQL validation refuses before any parser runs.
const REFUSED_INLINE: [&str; 4] = [
    r#"{ deviceId: "0190a000-0000-7000-8002-000000000001", name: "x", config: { http: { url: "http://a/", auth: { basic: { username: "u", password: "hunter2-outer", extra: 1 } } } } }"#,
    r#"{ deviceId: "0190a000-0000-7000-8002-000000000001", name: "x", config: { http: { url: "http://a/", auth: { bearerToken: 918273645, basic: { username: "u", password: "hunter2-outer" } } } } }"#,
    r#"{ deviceId: "0190a000-0000-7000-8002-000000000001", name: "x", password: "hunter2-outer", config: { push: true } }"#,
    r#"{ deviceId: "0190a000-0000-7000-8002-000000000001", name: "x", config: { http: { url: "http://a/", headers: [{ name: "Authorization", value: ["hunter2-outer"] }] } } }"#,
];

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../fixtures", scripts("devices"))
)]
async fn log_007_inline_check_inputs_of_the_wrong_shape_are_refused_without_quoting_them(
    pool: PgPool,
) {
    let schema = hardened_schema(&pool);
    for (index, literal) in REFUSED_INLINE.into_iter().enumerate() {
        let query = format!("mutation {{ createCheck(input: {literal}) {{ id }} }}");
        let (log, response) = captured(|| schema.execute(signed_in(&query))).await;
        assert!(!response.errors.is_empty(), "shape {index} was accepted");
        let body = serde_json::to_string(&response).unwrap();
        assert_absent(
            "hunter2-outer",
            &[&body, &format!("{:?}", response.errors), &log],
        );
    }
}
