//! Failed runs: the next sync backs off, waits out a rate limit, and a refused token asks the
//! operator to connect again with one warning (SRC-006, SRC-009, SRC-021).

use anima_hub::{engine::sources::Outcome, ids::SourceAccountId};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::json;
use sqlx::PgPool;

use super::{kit, set_projects, stub::Reply};
use crate::hub::start;

/// `(last_error_kind, failure_count, next_sync_at)` of `account`.
async fn after(pool: &PgPool, account: SourceAccountId) -> (Option<String>, i32, DateTime<Utc>) {
    sqlx::query_as(
        "SELECT last_error_kind, failure_count, next_sync_at FROM sources.accounts WHERE id = $1",
    )
    .bind(account)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn status(status: u16, headers: Vec<(&'static str, String)>) -> Reply {
    Reply {
        status,
        headers,
        body: "{}".to_owned(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn src_021_a_rate_limit_puts_the_next_sync_after_its_end(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.reply(
        "/pages/1",
        status(429, vec![("retry-after", "7200".to_owned())]),
    );
    assert_eq!(kit.run(id).await, Some(Outcome::Failed));
    assert_eq!(
        after(&pool, id).await,
        (
            Some("rate_limit".to_owned()),
            1,
            start() + TimeDelta::hours(2)
        )
    );

    let reset = start() + TimeDelta::minutes(5);
    kit.stub.reply(
        "/pages/1",
        status(
            403,
            vec![
                ("x-ratelimit-remaining", "0".to_owned()),
                ("x-ratelimit-reset", reset.timestamp().to_string()),
            ],
        ),
    );
    kit.run(id).await;
    assert_eq!(
        after(&pool, id).await,
        (
            Some("rate_limit".to_owned()),
            2,
            start() + TimeDelta::hours(1)
        ),
        "the backoff of two failures is later than the reset"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn src_021_failures_back_off_from_the_poll_interval(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.reply("/pages/1", status(500, Vec::new()));
    for (failures, minutes) in [(1, 30), (2, 60), (3, 120)] {
        assert_eq!(kit.run(id).await, Some(Outcome::Failed));
        let expected = (
            Some("provider".to_owned()),
            failures,
            start() + TimeDelta::minutes(minutes),
        );
        assert_eq!(after(&pool, id).await, expected);
    }
    let health = kit
        .hub
        .post(Some(&kit.token), "{ sourceAccounts { health } }", json!({}))
        .await;
    assert_eq!(health["data"]["sourceAccounts"][0]["health"], "STALLED");
    let warnings: Vec<String> =
        sqlx::query_scalar("SELECT message FROM events.event_log WHERE severity = 'warning'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        warnings,
        ["Source account Mine has stalled: its syncs keep failing."]
    );

    kit.stub.with(|s| s.replies.clear());
    kit.stub.set_pages(&[&[("a", "1")]]);
    assert_eq!(kit.run(id).await, Some(Outcome::Ok));
    assert_eq!(
        after(&pool, id).await,
        (None, 0, start() + TimeDelta::minutes(15))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn src_009_a_refused_token_needs_a_reconnect_and_warns_once(pool: PgPool) {
    let kit = kit(&pool).await;
    let (_, id) = kit.connect().await;
    set_projects(&pool, true).await;
    kit.stub.reply("/pages/1", status(401, Vec::new()));
    kit.run(id).await;
    kit.run(id).await;
    assert_eq!(after(&pool, id).await.0.as_deref(), Some("auth"));
    let health = kit
        .hub
        .post(Some(&kit.token), "{ sourceAccounts { health } }", json!({}))
        .await;
    assert_eq!(
        health["data"]["sourceAccounts"][0]["health"],
        "NEEDS_RECONNECT"
    );
    let warnings: i64 =
        sqlx::query_scalar("SELECT count(*) FROM events.event_log WHERE severity = 'warning'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(warnings, 1);
    let mints = kit
        .stub
        .paths()
        .iter()
        .filter(|p| p.starts_with("/app/"))
        .count();
    assert_eq!(
        mints, 3,
        "the connect's proof (not kept: Projects was off), the first run's, and a fresh one \
         after it was refused"
    );
}
