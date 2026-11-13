//! Signing in: identical refusals, the throttle and the records it leaves (SEC-002, LOG-005,
//! D-8).

use anima_hub::fleet::events::{Severity, Signal, Source};
use chrono::TimeDelta;
use serde_json::json;
use sqlx::PgPool;

use crate::hub::{CLIENT, Hub, PASSWORD, USERNAME, error, set_operator, start, wrong_credentials};

/// Five wrong passwords for the operator at `start()`.
async fn five_failures(hub: &Hub) {
    for _ in 0..5 {
        assert_eq!(
            error(&hub.login(USERNAME, "wrong password").await),
            wrong_credentials()
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_wrong_password_unknown_user_and_throttled_user_get_the_same_error(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());

    let wrong_password = hub.login(USERNAME, "wrong password").await;
    let unknown_user = hub.login("nobody", PASSWORD).await;
    let junk_user = hub.login("no\0body with spaces", PASSWORD).await;
    for _ in 0..4 {
        hub.login(USERNAME, "wrong password").await;
    }
    let throttled = hub.login(USERNAME, PASSWORD).await;

    for response in [&wrong_password, &unknown_user, &junk_user, &throttled] {
        assert_eq!(response["data"], json!(null), "{response}");
        assert_eq!(error(response), wrong_credentials());
        assert_eq!(
            response["errors"][0]["extensions"],
            json!({ "code": "UNAUTHENTICATED" })
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_throttle_starts_after_five_failures_and_ends_after_15_minutes(pool: PgPool) {
    set_operator(&pool).await;
    five_failures(&Hub::new(&pool, start())).await;

    let locked = Hub::new(
        &pool,
        start() + TimeDelta::minutes(15) - TimeDelta::seconds(1),
    );
    assert_eq!(
        error(&locked.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );

    let released = Hub::new(&pool, start() + TimeDelta::minutes(15));
    let response = released.login(USERNAME, PASSWORD).await;
    assert_eq!(
        response["data"]["login"]["operator"]["username"], USERNAME,
        "{response}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_four_failures_do_not_throttle(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for _ in 0..4 {
        hub.login(USERNAME, "wrong password").await;
    }
    assert!(hub.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_throttle_counts_failures_per_client_address(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    for name in ["a", "b", "c", "d", "e"] {
        hub.login(name, "guess").await;
    }
    assert_eq!(
        error(&hub.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_every_attempt_is_recorded(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    five_failures(&hub).await;
    hub.login(USERNAME, PASSWORD).await;
    let rows: Vec<(String, String, bool, bool)> = sqlx::query_as(
        "select username, host(client_addr), ok, refused from access.login_attempts order by id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 6);
    assert!(
        rows.iter()
            .all(|row| row.0 == USERNAME && row.1 == CLIENT && !row.2)
    );
    assert_eq!(
        rows.iter().filter(|row| row.3).count(),
        1,
        "the sixth was refused"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_an_operator_with_totp_cannot_sign_in_until_totp_is_checked(pool: PgPool) {
    set_operator(&pool).await;
    sqlx::query("update access.operators set totp_secret_enc = '\\x01'")
        .execute(&pool)
        .await
        .unwrap();
    let response = Hub::new(&pool, start()).login(USERNAME, PASSWORD).await;
    assert_eq!(error(&response), wrong_credentials());
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_login_and_logout_are_audited_with_name_and_client_address(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let token = hub.token().await;
    hub.post(Some(&token), "mutation { logout }", json!({}))
        .await;

    let operator_id: uuid::Uuid = sqlx::query_scalar("select id from access.operators")
        .fetch_one(&pool)
        .await
        .unwrap();
    let rows: Vec<(String, String, String, String, String, serde_json::Value)> = sqlx::query_as(
        "select action, operator_name, host(client_addr), target_type, target_id, change \
         from events.audit_log order by at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let entry = |action: &str| {
        (
            action.to_owned(),
            USERNAME.to_owned(),
            CLIENT.to_owned(),
            "operator".to_owned(),
            operator_id.to_string(),
            json!({}),
        )
    };
    assert_eq!(rows, [entry("login"), entry("logout")]);
}

#[sqlx::test(migrations = "./migrations")]
async fn log_005_sign_in_events_name_the_operator_and_never_the_password(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    let mut signals = hub.bus.subscribe();
    hub.login(USERNAME, "wrong password").await;
    hub.login(PASSWORD, "typed in the wrong field").await;
    hub.login(USERNAME, PASSWORD).await;

    let mut events = Vec::new();
    while let Ok(Signal::Event(event)) = signals.try_recv() {
        assert_eq!(event.source, Source::Access);
        events.push((event.severity, event.message));
    }
    assert_eq!(
        events,
        [
            (
                Severity::Warning,
                format!("Sign-in failed for operator operator from {CLIENT}.")
            ),
            (
                Severity::Warning,
                format!("Sign-in failed for an unknown username from {CLIENT}.")
            ),
            (Severity::Info, "Operator operator signed in.".to_owned()),
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_failures_from_another_address_do_not_lock_the_operator_out(pool: PgPool) {
    set_operator(&pool).await;
    let attacker = Hub::from_client(&pool, start(), "100.64.0.99");
    five_failures(&attacker).await;
    assert_eq!(
        error(&attacker.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );

    let operator = Hub::new(&pool, start()).login(USERNAME, PASSWORD).await;
    assert!(operator["data"]["login"]["token"].is_string(), "{operator}");
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_8_failures_spread_over_12_minutes_still_lock(pool: PgPool) {
    set_operator(&pool).await;
    for minute in [0, 3, 6, 9, 12] {
        let hub = Hub::new(&pool, start() + TimeDelta::minutes(minute));
        hub.login(USERNAME, "wrong password").await;
    }
    let after = Hub::new(&pool, start() + TimeDelta::minutes(13));
    assert_eq!(
        error(&after.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn sec_002_d_9_operator_set_clears_the_attempts_so_the_operator_gets_back_in(pool: PgPool) {
    set_operator(&pool).await;
    let hub = Hub::new(&pool, start());
    five_failures(&hub).await;
    assert_eq!(
        error(&hub.login(USERNAME, PASSWORD).await),
        wrong_credentials()
    );

    set_operator(&pool).await;
    assert!(hub.login(USERNAME, PASSWORD).await["data"]["login"]["token"].is_string());
}
