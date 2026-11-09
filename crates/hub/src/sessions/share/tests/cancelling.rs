//! A caller that goes away mid-`shareSession` (S-5, D-9, LOG-005): axum drops the resolver's
//! future when the client leaves or the hub drains. A share the registry already holds must
//! still get its keeper, end with its session, and leave a `share.end` for its `share.create`.
//! An opening that stops before it spawns the keeper (it panics, or the runtime drops it)
//! withdraws its share.

use std::{sync::Arc, thread, time::Duration};

use chrono::TimeDelta;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};

use super::{
    CLIENT, bench,
    ending::sharing,
    registry::{hash, share, share_id},
    share_audits,
};
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    sessions::{
        Actor,
        share::{
            Sharing,
            model::ShareAccess,
            opening::{Unkept, share_session},
            registry::Shares,
            terms::Terms,
        },
        tests::{
            fixture::{address, now, operator},
            tickets::session,
        },
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

/// The owner, sharing from [`CLIENT`] at `now()`.
fn owner() -> Actor {
    Actor {
        operator: operator(),
        client: address(CLIENT),
        at: now(),
    }
}

/// A view share for five minutes.
const fn five_minutes() -> Terms {
    Terms {
        access: ShareAccess::View,
        lifetime: TimeDelta::minutes(5),
        max_viewers: 3,
    }
}

/// A transaction that holds the audit table, so each insert of `share.create` waits until it
/// rolls back.
async fn audit_log_held(pool: &PgPool) -> Transaction<'static, Postgres> {
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("lock table events.audit_log in access exclusive mode")
        .execute(&mut *blocker)
        .await
        .unwrap();
    blocker
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
    let blocker = audit_log_held(&pool).await;
    // The client goes away mid-request: the resolver's future is dropped.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(300),
        share_session(&sharing(&bench), &owner(), bench.session, five_minutes()),
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

/// The runtime drops the opening task while `share.create` waits (the hub shuts down): the
/// guard `shareSession` armed withdraws the share it opened.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_5_a_share_whose_opening_the_runtime_drops_is_withdrawn(pool: PgPool) {
    let bench = bench(&pool).await;
    let blocker = audit_log_held(&pool).await;
    let options = (*pool.connect_options()).clone();
    let (live, of) = (bench.live.clone(), bench.session);
    // A runtime of its own, so the test can drop it with the opening task still waiting.
    let held = thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let held = runtime.block_on(async {
            let db = PgPoolOptions::new()
                .max_connections(2)
                .connect_with(options)
                .await
                .unwrap();
            let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
            let sharing = Sharing {
                db,
                bus: Bus::new(16),
                clock,
                live: live.clone(),
            };
            let by = owner();
            let opening = share_session(&sharing, &by, of, five_minutes());
            let cancelled = tokio::time::timeout(Duration::from_millis(300), opening).await;
            assert!(cancelled.is_err(), "the write was blocked");
            live.shares().hashes().len()
        });
        drop(runtime);
        held
    })
    .join()
    .unwrap();
    assert_eq!(held, 1, "held while share.create waited");
    let left = bench.live.shares().hashes().len();
    blocker.rollback().await.unwrap();
    assert_eq!(
        left, 0,
        "the runtime dropped the opening: the share is withdrawn"
    );
}
