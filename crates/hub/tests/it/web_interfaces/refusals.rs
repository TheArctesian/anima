//! Refusals: no operator, missing things and invalid input (API-003, K7, WEB-001, WEB-008).

use async_graphql::Request;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::harness::{
    ABSENT, DELETE, DISMISS, FINGERPRINT, HTTP_CHECK, PING_CHECK, PVE_01, accept_certificate_query,
    accept_suggestion_query, create, create_query, error_json, fixture_test, input, run,
    schema_for, update_query,
};

fn fields(error: &Value) -> Vec<&str> {
    assert_eq!(error["extensions"]["code"], "VALIDATION", "{error}");
    error["extensions"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect()
}

fn code(error: &Value) -> &str {
    error["extensions"]["code"].as_str().unwrap()
}

fixture_test! {
    async fn api_003_every_web_interface_mutation_without_an_operator_is_unauthenticated(pool: PgPool) {
        let schema = schema_for(&pool);
        let operations = [
            r#"mutation { createWebInterface(deviceId: "0190a000-0000-7000-8002-000000000001", input: { url: "http://a.lan/" }) { id } }"#.to_owned(),
            r#"mutation { updateWebInterface(id: "0190a000-0000-7000-8002-0000000000ff", input: { url: "http://a.lan/" }) { id } }"#.to_owned(),
            r#"mutation { deleteWebInterface(id: "0190a000-0000-7000-8002-0000000000ff") }"#.to_owned(),
            format!(r#"mutation {{ acceptWebInterfaceSuggestion(checkId: "{HTTP_CHECK}") {{ id }} }}"#),
            format!(r#"mutation {{ dismissWebInterfaceSuggestion(checkId: "{HTTP_CHECK}") }}"#),
            format!(r#"mutation {{ acceptWebCertificate(id: "{ABSENT}", fingerprintSha256: "{FINGERPRINT}") {{ id }} }}"#),
        ];
        for query in operations {
            let response = schema.execute(Request::new(query.clone())).await;
            assert_eq!(code(&error_json(&response)), "UNAUTHENTICATED", "{query}");
        }
        for table in ["web_interfaces.interfaces", "web_interfaces.dismissed", "events.audit_log"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
    }

    async fn k7_a_missing_device_interface_or_check_is_not_found_and_leaves_no_audit_entry(pool: PgPool) {
        let schema = schema_for(&pool);
        let responses = [
            run(&schema, &create_query(), json!({ "deviceId": ABSENT, "input": input("http://a.lan/") })).await,
            run(&schema, &update_query(), json!({ "id": ABSENT, "input": input("http://a.lan/") })).await,
            run(&schema, DELETE, json!({ "id": ABSENT })).await,
            run(&schema, &accept_certificate_query(), json!({ "id": ABSENT, "fingerprint": FINGERPRINT })).await,
            run(&schema, &accept_suggestion_query(), json!({ "checkId": ABSENT })).await,
            run(&schema, &accept_suggestion_query(), json!({ "checkId": PING_CHECK })).await,
            run(&schema, DISMISS, json!({ "checkId": ABSENT })).await,
        ];
        for response in responses {
            let error = error_json(&response);
            assert_eq!(code(&error), "NOT_FOUND", "{error}");
            assert!(!error["message"].as_str().unwrap().contains(ABSENT));
        }
        let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM events.audit_log")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(audits, 0);
    }

    async fn web_001_a_bad_url_is_validation_on_url_and_holds_no_input(pool: PgPool) {
        let schema = schema_for(&pool);
        let secret_url = "https://user:hunter2@pve-01.home.lan/";
        let response = run(
            &schema,
            &create_query(),
            json!({ "deviceId": PVE_01, "input": input(secret_url) }),
        )
        .await;
        let error = error_json(&response);
        assert_eq!(fields(&error), ["url"]);
        assert!(!error.to_string().contains("hunter2"));
        let id = create(&schema, "http://pve-01.home.lan/").await;
        let response = run(&schema, &update_query(), json!({ "id": id, "input": input("ftp://x/") })).await;
        assert_eq!(fields(&error_json(&response)), ["url"]);
    }

    async fn web_008_a_bad_fingerprint_or_an_http_interface_is_validation_on_fingerprint_sha256(pool: PgPool) {
        let schema = schema_for(&pool);
        let https = create(&schema, "https://pve-01.home.lan/").await;
        let plain = create(&schema, "http://pve-01.home.lan/").await;
        for (id, fingerprint) in [(&https, "abc123"), (&plain, FINGERPRINT)] {
            let response = run(
                &schema,
                &accept_certificate_query(),
                json!({ "id": id, "fingerprint": fingerprint }),
            )
            .await;
            assert_eq!(fields(&error_json(&response)), ["fingerprintSha256"]);
        }
    }
}
