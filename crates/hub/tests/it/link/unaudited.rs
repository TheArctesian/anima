//! A change the link made keeps its account: when the request goes away the audit entry is
//! still written, and when the entry cannot be written the operator hears that the link acted;
//! an invite is then cancelled, never shown (LOG-005, LOG-007, FRN-009).

use std::time::Duration;

use serde_json::{Value, json};
use sqlx::PgPool;

use super::{
    control::{FakeLink, INVITE, KEY, break_the_audit_log, code, hub_over, ok, one_device},
    devices::REVOKE,
    invites::CREATE,
};

const EXPIRES: &str = "2030-05-06T12:00:00.5Z";
const INVITE_ID: &str = "9f8e7d6c";

/// A link that makes invites, lists the one it made and cancels it.
fn inviting(request: &Value) -> String {
    match request["verb"].as_str() {
        Some("invites.make") => ok(&json!({ "invite": INVITE, "expires": EXPIRES })),
        Some("invites.list") => ok(&json!([
            { "id": "00000000", "name": "phone", "expires": "2030-05-06T11:00:00Z" },
            { "id": INVITE_ID, "name": "phone", "expires": EXPIRES },
        ])),
        Some("invites.cancel") => ok(&Value::Null),
        _ => one_device(),
    }
}

fn message(response: &Value) -> &str {
    response["errors"][0]["message"].as_str().unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_an_invite_without_its_audit_entry_is_cancelled_and_not_shown(pool: PgPool) {
    let link = FakeLink::start(inviting);
    let (hub, token) = hub_over(&pool, &link).await;
    break_the_audit_log(&pool).await;

    let text = hub
        .post_text(Some(&token), CREATE, json!({ "n": "phone" }))
        .await;

    let response: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(code(&response), "INTERNAL", "{response}");
    assert!(
        message(&response).contains("cancelled the invite"),
        "{response}"
    );
    assert!(!text.contains(INVITE), "{text}");
    assert_eq!(
        link.changes(),
        [
            json!({ "verb": "invites.make", "args": { "name": "phone" } }),
            json!({ "verb": "invites.list" }),
            json!({ "verb": "invites.cancel", "args": { "id": INVITE_ID } }),
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_revocation_without_its_audit_entry_says_the_link_revoked_it(pool: PgPool) {
    let link = FakeLink::start(|request| match request["verb"].as_str() {
        Some("devices.revoke") => ok(&json!({ "streamsClosed": 1 })),
        _ => one_device(),
    });
    let (hub, token) = hub_over(&pool, &link).await;
    break_the_audit_log(&pool).await;

    let response = hub.post(Some(&token), REVOKE, json!({ "k": KEY })).await;

    assert_eq!(code(&response), "INTERNAL", "{response}");
    assert_eq!(
        message(&response),
        "The link revoked the device, but the hub could not write its audit entry."
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_a_dropped_request_still_audits_the_revocation_the_link_made(pool: PgPool) {
    // The link takes 1 s for each answer; the client leaves once the revocation is sent.
    let link = FakeLink::slow(Duration::from_secs(1), |request| {
        match request["verb"].as_str() {
            Some("devices.revoke") => ok(&json!({ "streamsClosed": 1 })),
            _ => one_device(),
        }
    });
    let (hub, token) = hub_over(&pool, &link).await;
    let sent = async {
        while link.changes().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::select! {
        response = hub.post(Some(&token), REVOKE, json!({ "k": KEY })) => {
            panic!("the request ended before the link answered: {response}")
        }
        () = sent => {}
    }

    let audited = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let found: Option<String> = sqlx::query_scalar(
                "select change::text from events.audit_log \
                 where target_type = 'link' and action = 'delete'",
            )
            .fetch_optional(&pool)
            .await
            .unwrap();
            match found {
                Some(change) => return change,
                None => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("the audit entry of the revocation");
    assert!(audited.contains(r#""device": "laptop""#), "{audited}");
}
