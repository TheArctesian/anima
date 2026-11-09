//! A caller that goes away mid-`shareSession` (S-5, D-9, LOG-005): axum drops the resolver's
//! future when the client leaves or the hub drains. A share the registry already holds must
//! still get its keeper, end with its session, and leave a `share.end` for its `share.create`.
//! An opening that stops before it spawns the keeper (it panics, or the runtime drops it)
//! withdraws its share.

use std::time::Duration;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    CLIENT, bench,
    ending::sharing,
    registry::{hash, share, share_id},
    share_audits,
};
use crate::sessions::{
    Actor,
    share::{
        model::ShareAccess,
        opening::{Unkept, share_session},
        registry::Shares,
        terms::Terms,
    },
    tests::{
        fixture::{address, now, operator},
        tickets::session,
    },
};

/// Shares holding share 1 and share 2, and the guard of share 1 as its opening has it.
fn opening() -> (Shares, Unkept) {
    let shares = Shares::default();
    shares.open(hash(1), share(1, session(1))).unwrap();
    shares.open(hash(2), share(2, session(1))).unwrap();
    let unkept = Unkept::new(shares.clone(), share_id(1));
    (shares, unkept)
}

#[test]
fn s_5_a_share_whose_opening_stops_before_its_keeper_is_withdrawn() {
    let (shares, unkept) = opening();
    drop(unkept);
    assert!(shares.hashes() == [hash(2)], "only the other share stays");
}

#[tokio::test]
async fn s_5_a_share_whose_opening_panics_is_withdrawn() {
    let (shares, unkept) = opening();
    let opened = tokio::spawn(async move {
        let _unkept = unkept;
        panic!("the opening stopped before it spawned the keeper");
    })
    .await;
    assert!(opened.is_err_and(|stopped| stopped.is_panic()));
    assert!(shares.hashes() == [hash(2)], "only the other share stays");
}

#[test]
fn s_5_a_share_handed_to_its_keeper_stays_held() {
    let (shares, unkept) = opening();
    unkept.kept();
    assert_eq!(shares.hashes().len(), 2);
}

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
