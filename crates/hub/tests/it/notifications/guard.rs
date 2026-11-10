//! Every channel field needs a signed-in operator, and a read token may list channels but
//! never change one (API-003).

use async_graphql::{Request, Variables};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;

use super::harness::{
    DELETE, FIELDS, create_query, error_json, holds_no_secret, schema_for, update_query, webhook,
};
use crate::hub::{Hub, error, set_operator, start};

async fn stored(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM notifications.channels")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn api_003_channel_fields_without_an_operator_are_unauthenticated(pool: PgPool) {
    let schema = schema_for(&pool);
    let id = "0190a000-0000-7000-8fff-000000000001";
    let operations = [
        ("{ notificationChannels { id } }".to_owned(), json!({})),
        (create_query(), json!({ "input": webhook("Ops") })),
        (update_query(), json!({ "id": id, "input": webhook("Ops") })),
        (DELETE.to_owned(), json!({ "id": id })),
    ];
    for (query, variables) in operations {
        let request = Request::new(&query).variables(Variables::from_json(variables));
        let response = schema.execute(request).await;
        assert_eq!(
            error_json(&response)["extensions"]["code"],
            "UNAUTHENTICATED",
            "{query}"
        );
    }
    assert_eq!(stored(&pool).await, 0);
}

/// Only an API key may be read-only (migration 0010); it is spelled `anima_` + token.
pub async fn api_key(pool: &PgPool, byte: u8, scope: &str) -> String {
    let secret = [byte; 32];
    sqlx::query(
        "insert into access.api_tokens \
         (id, operator_id, token_hash, kind, name, display_prefix, scope, expires_at, created_at) \
         select gen_random_uuid(), id, $1, 'api_key', $4, 'anima_BQUFBQ', $5, $2, $3 \
         from access.operators",
    )
    .bind(Sha256::digest(secret).to_vec())
    .bind(start() + chrono::TimeDelta::days(1))
    .bind(start())
    .bind(format!("{scope} key"))
    .bind(scope)
    .execute(pool)
    .await
    .unwrap();
    format!("anima_{}", URL_SAFE_NO_PAD.encode(secret))
}

/// Through the hub as `main.rs` builds it: the channel roots and the loader are wired.
#[sqlx::test(migrations = "./migrations")]
async fn api_003_a_read_token_lists_channels_but_cannot_change_one(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    let input = json!({ "input": webhook("Ops hook") });
    let created = hub.post(Some(&token), &create_query(), input.clone()).await;
    assert_eq!(created["errors"], Value::Null, "{created}");
    holds_no_secret("the hub answer", &created.to_string());

    let reader = api_key(&pool, 5, "read").await;
    let writer = api_key(&pool, 6, "write").await;
    let list = format!("{{ notificationChannels {{ {FIELDS} }} }}");
    let listed = hub.post(Some(&reader), &list, json!({})).await;
    let channels = &listed["data"]["notificationChannels"];
    assert_eq!(
        channels[0]["id"],
        created["data"]["createNotificationChannel"]["id"]
    );
    assert_eq!(channels[0]["lastDelivery"], Value::Null);
    let id = channels[0]["id"].clone();
    for (query, variables) in [
        (create_query(), input),
        (
            update_query(),
            json!({ "id": id, "input": webhook("Renamed") }),
        ),
        (DELETE.to_owned(), json!({ "id": id })),
    ] {
        let refused = hub.post(Some(&reader), &query, variables).await;
        let read_only = (
            "This token may only read.".to_owned(),
            "FORBIDDEN".to_owned(),
        );
        assert_eq!(error(&refused), read_only);
    }
    assert_eq!(stored(&pool).await, 1);

    // R-100: a write API key may still change channels.
    let made = hub
        .post(
            Some(&writer),
            &create_query(),
            json!({ "input": webhook("Key hook") }),
        )
        .await;
    assert_eq!(made["errors"], Value::Null, "{made}");
    assert_eq!(stored(&pool).await, 2);
}
