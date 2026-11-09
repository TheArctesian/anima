//! A caller that goes away mid-`shareSession` (S-5, D-9, LOG-005): axum drops the resolver's
//! future when the client leaves or the hub drains. A share the registry already holds must
//! still get its keeper, end with its session, and leave a `share.end` for its `share.create`.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{CLIENT, bench, ending::sharing, share_audits};
use crate::sessions::{
    Actor,
    share::{model::ShareAccess, opening::share_session, terms::Terms},
    tests::fixture::{address, now, operator},
};

/// `(created, ended)` share ids in the audit log, each sorted.
async fn created_and_ended(pool: &PgPool) -> (Vec<String>, Vec<String>) {
    let (mut created, mut ended) = (Vec::new(), Vec::new());
    for (action, id, ..) in share_audits(pool).await {
        match action.as_str() {
            "create" => created.push(id),
            "end" => ended.push(id),
            _ => {}
        }
    }
    created.sort();
    ended.sort();
    (created, ended)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_5_a_cancelled_share_session_leaves_no_share_without_a_keeper(pool: PgPool) {
    let mut bench = bench(&pool).await;
    // Another transaction holds the audit table, so the insert of share.create waits.
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("lock table events.audit_log in access exclusive mode")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let by = Actor {
        operator: operator(),
        client: address(CLIENT),
        at: now(),
    };
    let terms = Terms {
        access: ShareAccess::View,
        lifetime: TimeDelta::minutes(5),
        max_viewers: 3,
    };
    // The client goes away mid-request: the resolver's future is dropped.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(300),
        share_session(&sharing(&bench), &by, bench.session, terms),
    )
    .await;
    assert!(
        cancelled.is_err(),
        "the write was blocked, so the call was cancelled"
    );
    blocker.rollback().await.unwrap();

    // The owner's session ends; every share ends with it.
    drop(bench.tunnel.take());
    let mut audits = (Vec::new(), Vec::new());
    for _ in 0..250 {
        audits = created_and_ended(&pool).await;
        if bench.live.shares().hashes().is_empty() && audits.0 == audits.1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        bench.live.shares().hashes().is_empty(),
        "a share with no keeper outlived its session"
    );
    let (created, ended) = audits;
    assert_eq!(created, ended, "each share.create has its share.end");
}
