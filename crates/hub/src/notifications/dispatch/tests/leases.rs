//! Leases: two dispatchers never send one row twice within a lease, a lease that ends lets
//! another dispatcher send, and a late outcome does not undo another's (D-16, NOT-001).

use std::collections::HashSet;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{NO_CONTENT, answering, channel, deliveries, dispatcher, queue, row, start, webhook};
use crate::notifications::{
    dispatch::{LEASE, claimed::Finished},
    egress::tests::local,
    store,
};

#[sqlx::test(migrations = "./migrations")]
async fn not_001_two_dispatchers_send_each_row_once(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let mut queued = HashSet::new();
    for _ in 0..24 {
        queued.insert(queue(&pool, hooks, None, t0).await);
    }
    let (first, second) = (dispatcher(&pool, egress.clone()), dispatcher(&pool, egress));

    let (a, b) = tokio::join!(first.tick(t0), second.tick(t0));
    let (c, d) = tokio::join!(first.tick(t0), second.tick(t0));

    assert_eq!(a + b + c + d, 24);
    server.settle().await;
    assert_eq!(server.connections(), 24);
    assert_eq!(deliveries(&pool).await.len(), 24);
    for id in queued {
        let state = row(&pool, id).await;
        assert_eq!((state.attempts, state.sent), (1, true));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_claimed_row_waits_out_its_lease_then_another_dispatcher_sends_it(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    // A dispatcher claims the row and stops before it sends.
    let lost = store::claim_due(&pool, t0, LEASE, 1).await.unwrap();
    assert_eq!(lost.len(), 1);
    let other = dispatcher(&pool, egress);

    let within = other.tick(t0 + LEASE - TimeDelta::seconds(1)).await;
    let after = other.tick(t0 + LEASE).await;

    assert_eq!((within, after), (0, 1));
    server.settle().await;
    assert_eq!(server.connections(), 1);
    assert!(row(&pool, queued).await.sent);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_late_outcome_leaves_a_row_another_dispatcher_finished(pool: PgPool) {
    let (working, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&working)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let slow = store::claim_due(&pool, t0, LEASE, 1).await.unwrap();
    // Its lease ends; another dispatcher sends the row.
    dispatcher(&pool, egress).tick(t0 + LEASE).await;
    let defaults = store::installation_settings(&pool).await.unwrap();

    // The first dispatcher's send fails afterwards (here: it does not trust the target's
    // certificate, so the handshake fails).
    let late = dispatcher(&pool, local())
        .finish(&slow[0], &defaults, t0 + LEASE + TimeDelta::seconds(1))
        .await
        .unwrap();

    assert!(matches!(late, Finished::Failed(_)));
    let state = row(&pool, queued).await;
    assert_eq!(
        (state.attempts, state.sent, state.given_up),
        (1, true, false)
    );
    let oks: Vec<bool> = deliveries(&pool).await.iter().map(|d| d.ok).collect();
    assert_eq!(oks, [true, false], "the late send is still logged");
}
