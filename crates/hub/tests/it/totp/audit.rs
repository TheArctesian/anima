//! The audit entries of the hub's own account commands (LOG-005, LOG-007): `operator create`
//! and `operator totp --disable` each write one, with the hub as the actor, this computer as
//! the client and no password or secret in the change. `operator set` writes none.

use anima_hub::fleet::access;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{code, create::create_with, enrol};
use crate::hub::{PASSWORD, USERNAME, start};

/// Each audit entry, oldest first: (operator name, action, target type, client, change).
async fn entries(pool: &PgPool) -> Vec<(String, String, String, String, Value)> {
    sqlx::query_as(
        "select operator_name, action, target_type, host(client_addr), change \
         from events.audit_log order by at, id",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

fn entry(action: &str, change: Value) -> (String, String, String, String, Value) {
    (
        "Anima".to_owned(),
        action.to_owned(),
        "operator".to_owned(),
        "127.0.0.1".to_owned(),
        change,
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_operator_create_audits_a_create_then_an_update(pool: PgPool) {
    for _ in 0..2 {
        let (text, secret) = super::create::new_secret();
        create_with(&pool, PASSWORD, &text, &code(&secret, start()))
            .await
            .unwrap();
    }
    let change = json!({ "username": USERNAME, "totp": true, "reason": "operator create" });
    assert_eq!(
        entries(&pool).await,
        [entry("create", change.clone()), entry("update", change)]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_operator_totp_disable_is_audited(pool: PgPool) {
    enrol(&pool).await;
    access::disable_totp(&pool, USERNAME, start())
        .await
        .unwrap();
    let disabled =
        json!({ "username": USERNAME, "totp": false, "reason": "operator totp --disable" });
    assert_eq!(entries(&pool).await, [entry("update", disabled)]);
}
