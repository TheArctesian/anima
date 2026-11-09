//! The keeper ends each share exactly once (S-5), with one cause, when expiry, `endShare` on
//! another thread and the end of the owner's tunnel race.

use std::{collections::BTreeSet, thread, time::Duration};

use chrono::TimeDelta;
use serde_json::Value;
use sqlx::PgPool;

use super::{bench, ending::shared_for, events, share_audits};
use crate::sessions::tests::fixture::{address, operator};

/// Waits up to five seconds for `count` `share.end` entries.
async fn share_ends(pool: &PgPool, count: usize) -> Vec<(String, Value)> {
    for _ in 0..250 {
        let ends: Vec<(String, Value)> = share_audits(pool)
            .await
            .into_iter()
            .filter(|(action, ..)| action == "end")
            .map(|(_, id, _, change)| (id, change))
            .collect();
        if ends.len() >= count {
            return ends;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the shares did not end");
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn ra_035_s_5_racing_causes_end_each_share_exactly_once(pool: PgPool) {
    const SHARES: i64 = 16;
    let mut bench = bench(&pool).await;
    let mut ids = Vec::new();
    for n in 0..SHARES {
        ids.push(shared_for(&bench, TimeDelta::milliseconds(2 * n)).await);
    }

    // The owner ends shares on another thread, newest first, while some expire.
    let shares = bench.live.shares().clone();
    let asked = ids.clone();
    let ender = thread::spawn(move || {
        let mut ended = BTreeSet::new();
        for id in asked.iter().rev() {
            if shares
                .end_by(*id, &operator(), address("100.64.0.9"))
                .is_ok()
            {
                ended.insert(id.to_string());
            }
            thread::sleep(Duration::from_millis(1));
        }
        ended
    });
    tokio::time::sleep(Duration::from_millis(10)).await;
    // Then the owner's tunnel ends, under whatever is still in flight.
    drop(bench.tunnel.take());
    while !ender.is_finished() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let ended_by_owner = ender.join().unwrap();

    let ends = share_ends(&pool, ids.len()).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let ends_later = share_ends(&pool, ids.len()).await;
    assert_eq!(ends_later.len(), ids.len(), "no second end: {ends_later:?}");
    let ended: BTreeSet<String> = ends.iter().map(|(id, _)| id.clone()).collect();
    let all: BTreeSet<String> = ids.iter().map(ToString::to_string).collect();
    assert_eq!(ended, all, "each share ended once");

    let mut by_cause = BTreeSet::new();
    for (id, change) in &ends {
        let cause = change["cause"].as_str().unwrap();
        assert!(
            ["expired", "ended", "session_ended"].contains(&cause),
            "{cause}"
        );
        if cause == "ended" {
            by_cause.insert(id.clone());
        }
    }
    assert_eq!(
        by_cause, ended_by_owner,
        "an endShare that was answered OK is the cause that stands, and only those"
    );
    let end_events = events(&pool)
        .await
        .iter()
        .filter(|(.., message)| message != "An operator shared a session.")
        .count();
    assert_eq!(end_events, ids.len(), "one event per end");
    assert!(bench.live.shares().hashes().is_empty());
}
