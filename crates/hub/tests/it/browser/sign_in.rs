//! `signInBrowser`: the cookie it sets, the body it never fills with the token, the Origin rule,
//! and the throttle and second factor it shares with `login` (D-3, D-4, A-4).

use axum::http::header;
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{PUBLIC, browser_hub, hash, post, sign_in_with};
use crate::{
    hub::{Hub, PASSWORD, USERNAME, error, set_operator, start, wrong_credentials},
    totp::{code, enrol},
};

#[sqlx::test(migrations = "./migrations")]
async fn d_3_sign_in_browser_sets_exactly_the_cookie_and_answers_without_the_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());

    let answer = sign_in_with(&hub, &[("origin", PUBLIC)], PASSWORD).await;

    let token = answer.token();
    assert_eq!(token.len(), 43, "a 32-byte token");
    assert_eq!(
        answer.set_cookies(),
        [format!(
            "__Host-anima={token}; Max-Age=604800; Path=/; Secure; HttpOnly; SameSite=Strict"
        )]
    );
    assert!(!answer.body.contains(&token), "{}", answer.body);
    let body = answer.json();
    let signed_in = &body["data"]["signInBrowser"];
    assert_eq!(
        signed_in["operator"],
        json!({ "username": USERNAME, "totpEnabled": false })
    );
    let expires_at: chrono::DateTime<chrono::Utc> =
        signed_in["expiresAt"].as_str().unwrap().parse().unwrap();
    assert_eq!(expires_at, start() + TimeDelta::days(7));
    let fields: Vec<&String> = signed_in.as_object().unwrap().keys().collect();
    assert_eq!(fields, ["expiresAt", "operator"]);
    let (kind, scope, stored_expiry): (String, String, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            "select kind, scope, expires_at from access.api_tokens where token_hash = $1",
        )
        .bind(hash(&token))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((kind.as_str(), scope.as_str()), ("browser", "write"));
    assert_eq!(stored_expiry, start() + TimeDelta::days(7));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_4_sign_in_browser_from_another_origin_is_forbidden_and_not_recorded(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let refusals: [&[(&str, &str)]; 6] = [
        &[],
        &[("origin", "https://evil.example")],
        &[("origin", "null")],
        &[("origin", "tauri://localhost")],
        &[("origin", "https://hub.anima.ing/")],
        &[("origin", PUBLIC), ("origin", PUBLIC)],
    ];
    for headers in refusals {
        let answer = sign_in_with(&hub, headers, PASSWORD).await;
        assert_eq!(answer.json()["data"], json!(null), "{headers:?}");
        assert_eq!(
            error(&answer.json()),
            (
                "Sign in from the hub's own address.".to_owned(),
                "FORBIDDEN".to_owned()
            ),
            "{headers:?}"
        );
        assert!(
            answer.headers.get(header::SET_COOKIE).is_none(),
            "{headers:?}"
        );
    }
    let counts: (i64, i64) = sqlx::query_as(
        "select (select count(*) from access.login_attempts), \
         (select count(*) from access.api_tokens)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (0, 0), "no attempt recorded, no token issued");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_4_failed_logins_throttle_sign_in_browser(pool: PgPool) {
    set_operator(&pool).await;
    let desktop = Hub::new(&pool, start());
    for _ in 0..5 {
        desktop.login(USERNAME, "wrong password").await;
    }
    let answer = sign_in_with(
        &browser_hub(&pool, start()),
        &[("origin", PUBLIC)],
        PASSWORD,
    )
    .await;
    assert_eq!(error(&answer.json()), wrong_credentials());
    assert!(answer.headers.get(header::SET_COOKIE).is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn d_4_failed_browser_sign_ins_throttle_login_and_are_recorded(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    for _ in 0..5 {
        let answer = sign_in_with(&hub, &[("origin", PUBLIC)], "wrong password").await;
        assert_eq!(error(&answer.json()), wrong_credentials());
    }
    let login = Hub::new(&pool, start()).login(USERNAME, PASSWORD).await;
    assert_eq!(error(&login), wrong_credentials());
    let failures: i64 =
        sqlx::query_scalar("select count(*) from access.login_attempts where not ok")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(failures, 6);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_4_sign_in_browser_needs_the_totp_code_once_enrolled(pool: PgPool) {
    let secret = enrol(&pool).await;
    let hub = browser_hub(&pool, start());
    let without = sign_in_with(&hub, &[("origin", PUBLIC)], PASSWORD).await;
    assert_eq!(error(&without.json()), wrong_credentials());
    assert!(without.headers.get(header::SET_COOKIE).is_none());

    let input = json!({ "input": {
        "username": USERNAME, "password": PASSWORD, "totpCode": code(&secret, start()) } });
    let with = post(&hub, &[("origin", PUBLIC)], super::SIGN_IN, input).await;
    assert_eq!(
        with.json()["data"]["signInBrowser"]["operator"]["totpEnabled"],
        true
    );
    assert_eq!(with.set_cookies().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_browser_sign_in_is_audited_as_a_login(pool: PgPool) {
    set_operator(&pool).await;
    super::browser_token(&browser_hub(&pool, start())).await;
    let actions: Vec<String> = sqlx::query_scalar("select action from events.audit_log")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(actions, ["login"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_the_desktop_app_still_signs_in_from_its_web_view(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let query = "mutation($input: LoginInput!) { login(input: $input) { token } }";
    let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });

    let answer = post(&hub, &[("origin", "tauri://localhost")], query, input).await;

    assert!(
        answer.json()["data"]["login"]["token"].is_string(),
        "{}",
        answer.body
    );
    assert_eq!(
        answer
            .headers
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "tauri://localhost"
    );
    assert!(answer.headers.get(header::SET_COOKIE).is_none());
    let public = post(&hub, &[("origin", PUBLIC)], "{ __typename }", json!({})).await;
    assert!(
        public
            .headers
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

/// Signed out, these documents select `signInBrowser` beside another root field or through a
/// root fragment: the backstop refuses each before it runs (API-003).
const HIDDEN_SIGN_INS: [&str; 3] = [
    "mutation($input: LoginInput!) { \
        signInBrowser(input: $input) { expiresAt } login(input: $input) { token } }",
    "mutation($input: LoginInput!) { \
        ... on Mutation { signInBrowser(input: $input) { expiresAt } } }",
    "mutation($input: LoginInput!) { ...F } \
        fragment F on Mutation { signInBrowser(input: $input) { expiresAt } }",
];

#[sqlx::test(migrations = "./migrations")]
async fn api_003_signed_out_only_one_sign_in_runs_and_never_through_a_fragment(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    for document in HIDDEN_SIGN_INS {
        let answer = post(&hub, &[("origin", PUBLIC)], document, input.clone()).await;
        assert_eq!(answer.json()["data"], json!(null), "{document}");
        assert_eq!(
            error(&answer.json()),
            crate::hub::unauthenticated(),
            "{document}"
        );
        assert!(
            answer.headers.get(header::SET_COOKIE).is_none(),
            "{document}"
        );
    }
    let counts: (i64, i64) = sqlx::query_as(
        "select (select count(*) from access.login_attempts), \
         (select count(*) from access.api_tokens)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (0, 0), "no attempt recorded, no token issued");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_4_a_hub_without_a_public_origin_signs_no_browser_in(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let input = json!({ "input": { "username": USERNAME, "password": PASSWORD } });
    let body = json!({ "query": super::SIGN_IN, "variables": input }).to_string();
    let request = axum::http::Request::post("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, PUBLIC)
        .body(axum::body::Body::from(body))
        .unwrap();
    let (_, text) = hub.send(request).await;
    let answer: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answer["errors"][0]["extensions"]["code"], "FORBIDDEN",
        "{answer}"
    );
}
