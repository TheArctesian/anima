//! Who may not use the link fields, and the errors of a hub with no link or a link that
//! fails (D-23, FRN-009, LOG-007).

use std::{fs, os::unix::fs::PermissionsExt as _};

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    control::{FakeLink, KEY, api_key, builder_token, code, hub_over, one_device, written},
    devices::REVOKE,
    invites::CREATE,
};
use crate::hub::{Hub, set_operator, start};

const DEVICES: &str = "{ linkDevices { key } }";

#[sqlx::test(migrations = "./migrations")]
async fn d_23_api_keys_and_builder_tokens_may_not_invite_or_revoke(pool: PgPool) {
    let link = FakeLink::start(|_| one_device());
    let (hub, token) = hub_over(&pool, &link).await;
    let tokens = [
        api_key(&hub, &token, "WRITE").await,
        api_key(&hub, &token, "READ").await,
        builder_token(&hub, &token).await,
    ];
    let before = written(&pool).await;

    for other in &tokens {
        for (query, variables) in [
            (CREATE, json!({ "n": "phone" })),
            (REVOKE, json!({ "k": KEY })),
        ] {
            let response = hub.post(Some(other), query, variables).await;
            assert_eq!(code(&response), "FORBIDDEN", "{query}: {response}");
        }
    }

    assert_eq!(link.changes(), Vec::<Value>::new());
    assert_eq!(written(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_without_anima_link_control_each_link_field_is_unavailable(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;

    for (query, variables) in [
        (DEVICES, json!({})),
        (CREATE, json!({ "n": "phone" })),
        (REVOKE, json!({ "k": KEY })),
    ] {
        let response = hub.post(Some(&token), query, variables).await;
        assert_eq!(code(&response), "UNAVAILABLE", "{query}: {response}");
        let message = response["errors"][0]["message"].as_str().unwrap();
        assert!(message.contains("ANIMA_LINK_CONTROL"), "{message}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_a_reply_over_one_mebibyte_is_unavailable_not_read(pool: PgPool) {
    let link =
        FakeLink::start(|_| format!("{{\"ok\":true,\"result\":\"{}\"}}\n", "a".repeat(1 << 20)));
    let (hub, token) = hub_over(&pool, &link).await;

    let response = hub.post(Some(&token), DEVICES, json!({})).await;

    assert_eq!(code(&response), "UNAVAILABLE", "{response}");
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_a_control_socket_others_can_reach_gets_no_request(pool: PgPool) {
    let link = FakeLink::start(|_| one_device());
    let (hub, token) = hub_over(&pool, &link).await;
    let folder = link.path.parent().unwrap();
    fs::set_permissions(folder, fs::Permissions::from_mode(0o755)).unwrap();

    let response = hub
        .post(Some(&token), CREATE, json!({ "n": "phone" }))
        .await;

    assert_eq!(code(&response), "UNAVAILABLE", "{response}");
    assert_eq!(link.changes(), Vec::<serde_json::Value>::new());
    fs::set_permissions(folder, fs::Permissions::from_mode(0o700)).unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_a_key_over_256_characters_is_refused_before_the_link(pool: PgPool) {
    let link = FakeLink::start(|_| one_device());
    let (hub, token) = hub_over(&pool, &link).await;

    let response = hub
        .post(Some(&token), REVOKE, json!({ "k": "k".repeat(257) }))
        .await;

    assert_eq!(code(&response), "VALIDATION", "{response}");
    assert_eq!(
        response["errors"][0]["extensions"]["fields"][0]["field"],
        "key"
    );
    assert_eq!(link.requests(), Vec::<Value>::new());
}
