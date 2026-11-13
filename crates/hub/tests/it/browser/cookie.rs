//! The `__Host-anima` cookie on `POST /graphql`: honoured only from the public origin, without
//! an `Authorization` header, and only for a browser token; cleared by `logout` (A-3, A-4, A-8).

use anima_hub::fleet::access;
use axum::http::header;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use super::{PUBLIC, browser_hub, browser_token, cookie, hash, me, post};
use crate::hub::{Hub, USERNAME, set_operator, start};

/// What `me` answers a signed-out request.
fn signed_out() -> Result<String, String> {
    Err("UNAUTHENTICATED".to_owned())
}

#[sqlx::test(migrations = "./migrations")]
async fn d_3_the_cookie_signs_in_only_with_the_public_origin(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);

    let same = me(&hub, &[("cookie", &cookie), ("origin", PUBLIC)]).await;
    assert_eq!(same.as_deref(), Ok(USERNAME));
    let refusals: [&[(&str, &str)]; 7] = [
        &[("cookie", &cookie)],
        &[("cookie", &cookie), ("origin", "https://evil.example")],
        &[
            ("cookie", &cookie),
            ("origin", "https://device.net.anima.ing"),
        ],
        &[("cookie", &cookie), ("origin", "null")],
        &[("cookie", &cookie), ("origin", "tauri://localhost")],
        &[("cookie", &cookie), ("origin", "http://hub.anima.ing")],
        &[("cookie", &cookie), ("origin", PUBLIC), ("origin", PUBLIC)],
    ];
    for headers in refusals {
        assert_eq!(me(&hub, headers).await, signed_out(), "{headers:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_an_authorization_header_hides_the_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);
    for authorization in ["Bearer junk", "Basic b3A6cHc=", ""] {
        let headers = [
            ("cookie", cookie.as_str()),
            ("origin", PUBLIC),
            ("authorization", authorization),
        ];
        assert_eq!(me(&hub, &headers).await, signed_out(), "{authorization:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_two_cookies_of_the_name_sign_nobody_in(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let token = browser_token(&hub).await;
    let twice = format!("__Host-anima={token}; __Host-anima={token}");
    assert_eq!(
        me(&hub, &[("cookie", &twice), ("origin", PUBLIC)]).await,
        signed_out()
    );
    let quoted = format!("theme=dark; __Host-anima=\"{token}\"");
    let answer = me(&hub, &[("cookie", &quoted), ("origin", PUBLIC)]).await;
    assert_eq!(answer.as_deref(), Ok(USERNAME));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_a_browser_token_is_no_bearer(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let token = browser_token(&hub).await;
    let bearer = format!("Bearer {token}");
    assert_eq!(me(&hub, &[("authorization", &bearer)]).await, signed_out());
    let marked = [("authorization", bearer.as_str()), ("origin", PUBLIC)];
    assert_eq!(me(&hub, &marked).await, signed_out());
    let tunnel = access::operator_for_token(&pool, &token, start()).await;
    assert!(matches!(tunnel, Err(access::Error::Unauthenticated)));
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_operator_for_token_takes_only_a_login_token(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let secret = URL_SAFE_NO_PAD.encode([7_u8; 32]);
    for scope in ["read", "write"] {
        sqlx::query("delete from access.api_tokens")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "insert into access.api_tokens (id, operator_id, token_hash, kind, made_for, name, \
             display_prefix, scope, expires_at, created_at) \
             select gen_random_uuid(), id, $1, 'api_key', 'user', 'script', 'anima_BwcHBw', $2, $3, $4 \
             from access.operators",
        )
        .bind(hash(&secret))
        .bind(scope)
        .bind(start() + TimeDelta::days(1))
        .bind(start())
        .execute(&pool)
        .await
        .unwrap();
        let key = format!("anima_{secret}");
        let bearer = format!("Bearer {key}");
        let as_bearer = me(&hub, &[("authorization", &bearer)]).await;
        assert_eq!(as_bearer.as_deref(), Ok(USERNAME), "a valid {scope} key");
        let tunnel = access::operator_for_token(&pool, &key, start()).await;
        assert!(
            matches!(tunnel, Err(access::Error::Unauthenticated)),
            "{scope} key"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_3_a_login_token_is_no_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let hub = browser_hub(&pool, start());
    let headers = [("cookie", cookie(&token)), ("origin", PUBLIC.to_owned())];
    let headers = headers
        .each_ref()
        .map(|(name, value)| (*name, value.as_str()));
    assert_eq!(me(&hub, &headers).await, signed_out());
    let key = format!("anima_{token}");
    let headers = [("cookie", cookie(&key)), ("origin", PUBLIC.to_owned())];
    let headers = headers
        .each_ref()
        .map(|(name, value)| (*name, value.as_str()));
    assert_eq!(
        me(&hub, &headers).await,
        signed_out(),
        "a cookie carries no key"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_8_logout_deletes_the_browser_token_and_clears_the_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let token = browser_token(&hub).await;
    let headers = [("cookie", cookie(&token)), ("origin", PUBLIC.to_owned())];
    let headers = headers
        .each_ref()
        .map(|(name, value)| (*name, value.as_str()));

    let answer = post(&hub, &headers, "mutation { logout }", json!({})).await;

    assert_eq!(answer.json(), json!({ "data": { "logout": true } }));
    assert_eq!(
        answer.set_cookies(),
        ["__Host-anima=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Strict"]
    );
    let left: i64 =
        sqlx::query_scalar("select count(*) from access.api_tokens where token_hash = $1")
            .bind(hash(&token))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(left, 0);
    assert_eq!(me(&hub, &headers).await, signed_out());
}

#[sqlx::test(migrations = "./migrations")]
async fn a_8_a_desktop_logout_sets_no_cookie(pool: PgPool) {
    set_operator(&pool).await;
    let token = Hub::new(&pool, start()).token().await;
    let hub = browser_hub(&pool, start());
    let bearer = format!("Bearer {token}");
    let answer = post(
        &hub,
        &[("authorization", &bearer)],
        "mutation { logout }",
        json!({}),
    )
    .await;
    assert_eq!(answer.json(), json!({ "data": { "logout": true } }));
    assert!(answer.headers.get(header::SET_COOKIE).is_none());
}

/// R-108: a browser token gets no upload URLs, even from the public origin.
#[sqlx::test(migrations = "./migrations")]
async fn r_108_a_browser_token_gets_no_upload_urls(pool: PgPool) {
    set_operator(&pool).await;
    let hub = browser_hub(&pool, start());
    let cookie = cookie(&browser_token(&hub).await);
    let urls = "mutation { buildUploadUrls(id: \"0190a000-0000-7000-8009-000000000001\", \
                files: []) { url } }";
    let answer = post(
        &hub,
        &[("cookie", &cookie), ("origin", PUBLIC)],
        urls,
        json!({}),
    )
    .await
    .json();
    assert_eq!(
        answer["errors"][0]["extensions"]["code"], "FORBIDDEN",
        "{answer}"
    );
    assert!(answer["data"].is_null(), "{answer}");
}
