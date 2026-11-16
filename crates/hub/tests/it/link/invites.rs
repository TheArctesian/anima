//! `createLinkInvite` through the hub: the invite is shown once, with `no-store`, and it reaches
//! no audit entry, event or log line (LNK-003, LOG-005, LOG-007).

use anima_hub::fleet::events::process_log;
use serde_json::json;
use sqlx::PgPool;

use super::control::{FakeLink, INVITE, code, hub_over, ok, refusal, written};
use crate::logs::Capture;

pub const CREATE: &str =
    "mutation($n: String!) { createLinkInvite(name: $n) { invite expiresAt } }";

fn made() -> String {
    ok(&json!({ "invite": INVITE, "expires": "2030-05-06T12:00:00.5Z" }))
}

#[sqlx::test(migrations = "./migrations")]
async fn lnk_003_create_link_invite_shows_the_invite_once_and_writes_no_copy(pool: PgPool) {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("debug", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();
    let link = FakeLink::start(|_| made());
    let (hub, token) = hub_over(&pool, &link).await;

    let (headers, response) = hub
        .post_with_headers(&token, CREATE, json!({ "n": "phone" }))
        .await;

    assert!(response["errors"].is_null(), "{response}");
    let invite = &response["data"]["createLinkInvite"];
    assert_eq!(invite["invite"], INVITE);
    assert_eq!(invite["expiresAt"], "2030-05-06T12:00:00.500+00:00");
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(
        link.changes(),
        [json!({ "verb": "invites.make", "args": { "name": "phone" } })]
    );
    let stored = written(&pool).await;
    assert!(stored.contains(r#""device": "phone""#), "{stored}");
    assert!(stored.contains("link create"), "{stored}");
    assert!(stored.contains("access Made a link invite for the device phone."));
    let logged = capture.text();
    for text in [&stored, &logged] {
        assert!(!text.contains(INVITE), "{text}");
        assert!(!text.contains("ABCDEFGHIJ"), "{text}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_a_name_the_link_refuses_is_a_validation_error_on_name(pool: PgPool) {
    let link = FakeLink::start(|_| refusal("bad_name"));
    let (hub, token) = hub_over(&pool, &link).await;
    let before = written(&pool).await;

    let response = hub
        .post(Some(&token), CREATE, json!({ "n": "\u{200b}" }))
        .await;

    assert_eq!(code(&response), "VALIDATION", "{response}");
    assert_eq!(
        response["errors"][0]["extensions"]["fields"][0]["field"],
        "name"
    );
    assert_eq!(written(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn frn_009_a_name_over_64_characters_is_refused_before_the_link(pool: PgPool) {
    let link = FakeLink::start(|_| made());
    let (hub, token) = hub_over(&pool, &link).await;

    for name in [String::new(), "x".repeat(65), "a\nb".to_owned()] {
        let response = hub.post(Some(&token), CREATE, json!({ "n": name })).await;
        assert_eq!(code(&response), "VALIDATION", "{response}");
    }
    assert_eq!(link.changes(), Vec::<serde_json::Value>::new());
}
