//! The outbox: claims, finishing a row and the 7-day clean-up (D-16, K2).

use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{channel, count, now, runtime_pool};
use crate::{
    fleet::notifications::{
        NewOutbox, NotificationKind, Outbox, OutboxId, delete_finished_outbox, store as sql,
    },
    ids::ChannelId,
};

async fn queue(pool: &PgPool, channel: ChannelId, at: DateTime<Utc>) -> OutboxId {
    let id = OutboxId::from(Uuid::now_v7());
    let row = NewOutbox {
        id,
        channel,
        check: None,
        kind: NotificationKind::Test,
        payload: serde_json::json!({ "device": "pve-01" }),
    };
    let mut conn = pool.acquire().await.unwrap();
    sql::insert_outbox(&mut conn, &row, at).await.unwrap();
    id
}

/// The claim that holds row `id` of `channel` now: its token is the row's `next_attempt_at`.
async fn holding(pool: &PgPool, channel: ChannelId, id: OutboxId) -> Outbox {
    let lease_end: DateTime<Utc> =
        sqlx::query_scalar("SELECT next_attempt_at FROM notifications.outbox WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    Outbox {
        id,
        channel,
        check: None,
        kind: NotificationKind::Test,
        payload: serde_json::json!({}),
        created_at: lease_end,
        attempts: 1,
        lease_end,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn not_001_two_claimers_take_disjoint_rows_and_none_twice(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await - TimeDelta::minutes(1);
    for _ in 0..20 {
        queue(&pool, ops, at).await;
    }
    let lease = TimeDelta::minutes(5);
    let later = at + TimeDelta::minutes(2);

    let (first, second) = tokio::join!(
        sql::claim_due(&runtime, later, lease, 10),
        sql::claim_due(&runtime, later, lease, 10),
    );
    let (first, second) = (first.unwrap(), second.unwrap());
    let rest = sql::claim_due(&runtime, later, lease, 100).await.unwrap();
    let again = sql::claim_due(&runtime, later, lease, 100).await.unwrap();

    let ids: Vec<OutboxId> = [first, second, rest]
        .iter()
        .flatten()
        .map(|row| row.id)
        .collect();
    assert_eq!(ids.len(), 20);
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 20);
    assert_eq!(again.len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn not_001_a_claim_skips_rows_another_transaction_holds_instead_of_waiting(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await - TimeDelta::minutes(1);
    let mut ids = Vec::new();
    for _ in 0..6 {
        ids.push(queue(&pool, ops, at).await);
    }
    let mut holder = pool.begin().await.unwrap();
    let held: Vec<OutboxId> =
        sqlx::query_scalar("SELECT id FROM notifications.outbox ORDER BY id LIMIT 3 FOR UPDATE")
            .fetch_all(&mut *holder)
            .await
            .unwrap();
    assert_eq!(held, ids[..3]);

    // Without SKIP LOCKED this would wait for the holder, which ends only after the assert.
    let claimed = tokio::time::timeout(
        Duration::from_secs(10),
        sql::claim_due(
            &runtime,
            at + TimeDelta::minutes(2),
            TimeDelta::minutes(5),
            100,
        ),
    )
    .await
    .expect("the claim waited for rows another transaction holds")
    .unwrap();
    holder.rollback().await.unwrap();

    let claimed: HashSet<OutboxId> = claimed.iter().map(|row| row.id).collect();
    assert_eq!(claimed, ids[3..].iter().copied().collect());
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_a_claim_skips_future_and_finished_rows_and_a_lease_expires(pool: PgPool) {
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await;
    let due = queue(&pool, ops, at).await;
    let future = queue(&pool, ops, at + TimeDelta::hours(1)).await;
    let sent = queue(&pool, ops, at).await;
    let given_up = queue(&pool, ops, at).await;
    let (sent, given_up) = (
        holding(&pool, ops, sent).await,
        holding(&pool, ops, given_up).await,
    );
    assert!(sql::mark_sent(&pool, &sent, at).await.unwrap());
    assert!(sql::give_up(&pool, &given_up, at).await.unwrap());

    let claimed = sql::claim_due(&pool, at, TimeDelta::minutes(5), 10)
        .await
        .unwrap();
    let during = sql::claim_due(&pool, at + TimeDelta::minutes(1), TimeDelta::minutes(5), 10)
        .await
        .unwrap();
    let after = sql::claim_due(&pool, at + TimeDelta::minutes(6), TimeDelta::minutes(5), 10)
        .await
        .unwrap();

    assert_eq!(claimed.iter().map(|row| row.id).collect::<Vec<_>>(), [due]);
    assert_eq!(during.len(), 0);
    assert_eq!(after.iter().map(|row| row.id).collect::<Vec<_>>(), [due]);
    let hour_on = sql::claim_due(&pool, at + TimeDelta::hours(2), TimeDelta::minutes(5), 10)
        .await
        .unwrap();
    assert!(hour_on.iter().any(|row| row.id == future));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_each_claim_counts_a_try_and_a_finished_row_stays_finished(pool: PgPool) {
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await;
    queue(&pool, ops, at).await;
    let lease = TimeDelta::minutes(5);
    let first = sql::claim_due(&pool, at, lease, 1).await.unwrap();

    assert!(
        sql::reschedule(&pool, &first[0], at + TimeDelta::minutes(1))
            .await
            .unwrap()
    );
    let retry = sql::claim_due(&pool, at + TimeDelta::minutes(1), lease, 1)
        .await
        .unwrap();
    assert_eq!((first[0].attempts, retry[0].attempts), (1, 2));
    assert!(sql::give_up(&pool, &retry[0], at).await.unwrap());

    assert!(!sql::mark_sent(&pool, &retry[0], at).await.unwrap());
    assert!(!sql::reschedule(&pool, &retry[0], at).await.unwrap());
    assert!(!sql::give_up(&pool, &retry[0], at).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_claim_whose_lease_ended_changes_nothing_once_another_claim_holds_the_row(
    pool: PgPool,
) {
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await;
    let id = queue(&pool, ops, at).await;
    let lease = TimeDelta::minutes(5);
    let stale = sql::claim_due(&pool, at, lease, 1).await.unwrap();
    let current = sql::claim_due(&pool, at + lease, lease, 1).await.unwrap();

    assert!(!sql::mark_sent(&pool, &stale[0], at).await.unwrap());
    assert!(!sql::reschedule(&pool, &stale[0], at).await.unwrap());
    assert!(!sql::give_up(&pool, &stale[0], at).await.unwrap());
    let state: (DateTime<Utc>, bool) = sqlx::query_as(
        "SELECT next_attempt_at, sent_at IS NULL AND given_up_at IS NULL          FROM notifications.outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, (current[0].lease_end, true));
    assert!(
        sql::mark_sent(&pool, &current[0], at + lease)
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn not_004_clean_up_deletes_only_rows_finished_more_than_7_days_ago(pool: PgPool) {
    let runtime = runtime_pool(&pool).await;
    let ops = channel(&pool, "ops").await;
    let at = now(&pool).await;
    let old_sent = queue(&pool, ops, at).await;
    let old_given_up = queue(&pool, ops, at).await;
    let recent = queue(&pool, ops, at).await;
    queue(&pool, ops, at - TimeDelta::days(30)).await;
    let old_sent = holding(&pool, ops, old_sent).await;
    let old_given_up = holding(&pool, ops, old_given_up).await;
    let recent = holding(&pool, ops, recent).await;
    sql::mark_sent(&pool, &old_sent, at - TimeDelta::days(8))
        .await
        .unwrap();
    sql::give_up(&pool, &old_given_up, at - TimeDelta::days(8))
        .await
        .unwrap();
    sql::mark_sent(&pool, &recent, at - TimeDelta::days(6))
        .await
        .unwrap();

    let deleted = delete_finished_outbox(&runtime, at).await.unwrap();

    assert_eq!(deleted, 2);
    assert_eq!(count(&pool, "outbox").await, 2);
}
