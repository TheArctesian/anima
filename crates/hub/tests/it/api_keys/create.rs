//! `createApiKey` and `apiKeys`: the secret's spelling, shown once, stored as a hash, and the
//! name and expiry rules (INT-003, D-23, P3-004).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Months, TimeDelta};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    LIST, ME, body, coded, count, create, hash, input, instant, made, rows, secret, signed_in,
};
use crate::hub::{Hub, start};

#[sqlx::test(migrations = "./migrations")]
async fn int_003_d_23_the_secret_is_anima_and_43_base64url_characters_shown_once(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let created = made(&hub, &token, "backup script", "READ").await;
    let secret = created["secret"].as_str().unwrap();
    assert_eq!(secret.len(), 49, "{secret}");
    let decoded = URL_SAFE_NO_PAD.decode(body(secret)).unwrap();
    assert_eq!(decoded.len(), 32);
    assert_eq!(URL_SAFE_NO_PAD.encode(&decoded), body(secret), "canonical");
    let key = &created["key"];
    assert_eq!(key["prefix"], json!(&secret[..12]));
    assert_eq!(
        (&key["name"], &key["scope"]),
        (&json!("backup script"), &json!("READ"))
    );
    assert_eq!(instant(&key["createdAt"]), start());
    assert_eq!(instant(&key["expiresAt"]), start() + TimeDelta::days(30));
    assert_eq!(key["lastUsedAt"], Value::Null);

    let listed = hub.post_text(Some(&token), LIST, json!({})).await;
    assert!(!listed.contains(body(secret)), "{listed}");
    let listed: Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(listed["data"]["apiKeys"], json!([key]));
    let asked = hub
        .post(Some(&token), "{ apiKeys { secret } }", json!({}))
        .await;
    assert!(asked["errors"].is_array(), "no field holds it: {asked}");

    let other = made(&hub, &token, "second", "WRITE").await;
    assert_ne!(other["secret"], created["secret"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn int_003_d_23_a_key_signs_in_and_its_use_is_listed(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let secret = secret(&hub, &token, "metrics", "READ").await;
    let later = Hub::new(&pool, start() + TimeDelta::minutes(5));
    let me = later.post(Some(&secret), ME, json!({})).await;
    assert_eq!(me["data"]["me"]["username"], "operator", "{me}");
    let listed = later.post(Some(&token), LIST, json!({})).await;
    let used = &listed["data"]["apiKeys"][0]["lastUsedAt"];
    assert_eq!(instant(used), start() + TimeDelta::minutes(5));

    for spelled in [body(&secret).to_owned(), secret.to_uppercase()] {
        let response = later.post(Some(&spelled), ME, json!({})).await;
        assert_eq!(coded(&response).0, "UNAUTHENTICATED", "{spelled}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn int_003_p3_004_only_the_sha_256_of_the_secret_is_stored(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let secret = secret(&hub, &token, "deploy", "WRITE").await;
    hub.post(Some(&secret), ME, json!({})).await;
    let mut stored = Vec::new();
    for table in [
        "access.api_tokens",
        "access.operators",
        "access.login_attempts",
        "events.audit_log",
        "events.event_log",
    ] {
        stored.extend(rows(&pool, table).await);
    }
    for row in &stored {
        assert!(!row.contains(body(&secret)), "{row}");
    }
    let (stored_hash, kind, prefix): (Vec<u8>, String, String) = sqlx::query_as(
        "select token_hash, kind, display_prefix from access.api_tokens where kind = 'api_key'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored_hash, hash(&secret));
    assert_eq!((kind.as_str(), prefix.as_str()), ("api_key", &secret[..12]));
}

/// The `expiresAt` refusal, and that no key was made.
async fn refused_expiry(hub: &Hub, pool: &PgPool, token: &str, expires_at: Value) {
    let input = json!({ "name": "late", "scope": "READ", "expiresAt": expires_at });
    let response = create(hub, token, input).await;
    let (code, fields) = coded(&response);
    assert_eq!(code, "VALIDATION", "{expires_at}: {response}");
    assert_eq!(fields[0]["field"], "expiresAt", "{response}");
    assert_eq!(
        count(pool, "access.api_tokens where kind = 'api_key'").await,
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_the_expiry_is_in_the_future_and_at_most_one_year_away(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    let year = start().checked_add_months(Months::new(12)).unwrap();
    for refused in [
        start() - TimeDelta::days(1),
        start(),
        year + TimeDelta::seconds(1),
        start() + TimeDelta::days(3650),
    ] {
        refused_expiry(&hub, &pool, &token, json!(refused)).await;
    }
    let missing = create(&hub, &token, json!({ "name": "late", "scope": "READ" })).await;
    assert!(missing["errors"].is_array(), "{missing}");
    assert_eq!(
        count(&pool, "access.api_tokens where kind = 'api_key'").await,
        0
    );

    for (name, allowed) in [("soon", start() + TimeDelta::seconds(1)), ("a year", year)] {
        let input = json!({ "name": name, "scope": "WRITE", "expiresAt": allowed });
        let key = &create(&hub, &token, input).await["data"]["createApiKey"]["key"];
        assert_eq!(instant(&key["expiresAt"]), allowed, "{name}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_the_name_is_1_to_64_plain_characters(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    for refused in [
        String::new(),
        "   ".to_owned(),
        "k".repeat(65),
        "a\u{200b}b".to_owned(),
        "a\u{202e}b".to_owned(),
        "a\nb".to_owned(),
    ] {
        let response = create(&hub, &token, input(&refused, "READ")).await;
        let (code, fields) = coded(&response);
        assert_eq!(code, "VALIDATION", "{refused:?}: {response}");
        assert_eq!(fields[0]["field"], "name");
    }
    let longest = "ü".repeat(64);
    assert_eq!(
        made(&hub, &token, &longest, "READ").await["key"]["name"],
        json!(longest)
    );
    let trimmed = made(&hub, &token, "  nightly  ", "READ").await;
    assert_eq!(trimmed["key"]["name"], "nightly");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_23_a_taken_name_is_a_conflict_on_the_name(pool: PgPool) {
    let (hub, token) = signed_in(&pool).await;
    made(&hub, &token, "deploy", "READ").await;
    let again = create(&hub, &token, input("deploy", "WRITE")).await;
    let (code, fields) = coded(&again);
    assert_eq!(code, "CONFLICT", "{again}");
    assert_eq!(fields[0]["field"], "name", "{again}");
    assert_eq!(again["data"], Value::Null);
    assert_eq!(
        count(&pool, "access.api_tokens where kind = 'api_key'").await,
        1
    );
    assert_eq!(
        count(&pool, "events.audit_log where target_type = 'api_key'").await,
        1,
        "the refused one left no entry"
    );
}
