//! Leases: two dispatchers never send one row twice within a lease, a lease that ends lets
//! another dispatcher send, a late outcome changes nothing in a row another claim holds, and a
//! send whose outcome is never written ends with the row's tries (D-16, NOT-001).

use std::collections::HashSet;

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    NO_CONTENT, answering, channel, deliveries, dispatcher, queue, row, start, warnings, webhook,
};
use crate::{
    notifications::{
        DeliveryReason,
        dispatch::{LEASE, claimed::Finished},
        egress::{Egress, tests::local},
        store,
    },
    targets::AddressPolicy,
};

/// An egress that refuses every loopback target: each send fails as `TARGET_NOT_PERMITTED`,
/// a permanent failure.
fn refusing() -> Egress {
    Egress::new(AddressPolicy::default())
}

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
async fn d_16_a_late_retryable_failure_leaves_a_row_another_dispatcher_sent(pool: PgPool) {
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
        (2, true, false)
    );
    let oks: Vec<bool> = deliveries(&pool).await.iter().map(|d| d.ok).collect();
    assert_eq!(oks, [true, false], "the late send is still logged");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_late_permanent_failure_leaves_a_row_another_dispatcher_sent(pool: PgPool) {
    let (working, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&working)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let slow = store::claim_due(&pool, t0, LEASE, 1).await.unwrap();
    dispatcher(&pool, egress).tick(t0 + LEASE).await;
    let defaults = store::installation_settings(&pool).await.unwrap();

    let late = dispatcher(&pool, refusing())
        .finish(&slow[0], &defaults, t0 + LEASE + TimeDelta::seconds(1))
        .await
        .unwrap();

    assert!(matches!(late, Finished::Failed(_)));
    let state = row(&pool, queued).await;
    assert_eq!((state.sent, state.given_up), (true, false));
    assert_eq!(
        warnings(&pool).await.len(),
        0,
        "no Warning for a row that was sent"
    );
    let logged = deliveries(&pool).await;
    let reasons: Vec<_> = logged.iter().map(|d| d.reason).collect();
    assert_eq!(reasons, [None, Some(DeliveryReason::TargetNotPermitted)]);
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_late_reschedule_leaves_another_dispatchers_lease_whole(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let defaults = store::installation_settings(&pool).await.unwrap();
    let a = store::claim_due(&pool, t0, LEASE, 1).await.unwrap();
    // A stalls past its lease; B claims the row and is sending under its own lease.
    let b_now = t0 + LEASE;
    let b = store::claim_due(&pool, b_now, LEASE, 1).await.unwrap();
    // A's send fails with a retryable failure (it does not trust the target), and A writes at
    // its own tick's time, which would make the row due at t0 + 1 min.
    dispatcher(&pool, local())
        .finish(&a[0], &defaults, t0)
        .await
        .unwrap();

    let inside_b = dispatcher(&pool, egress.clone())
        .tick(b_now + TimeDelta::seconds(1))
        .await;
    dispatcher(&pool, egress)
        .finish(&b[0], &defaults, b_now)
        .await
        .unwrap();

    assert_eq!(inside_b, 0, "a third claimer took the row inside B's lease");
    server.settle().await;
    assert_eq!(server.bodies().len(), 1, "only B's send reached the target");
    let state = row(&pool, queued).await;
    assert_eq!((state.attempts, state.sent), (2, true));
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_row_two_claimers_give_up_warns_once(pool: PgPool) {
    let (server, _) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    let defaults = store::installation_settings(&pool).await.unwrap();
    let a = store::claim_due(&pool, t0, LEASE, 1).await.unwrap();
    let b = store::claim_due(&pool, t0 + LEASE, LEASE, 1).await.unwrap();

    for (claim, at) in [
        (&b[0], t0 + LEASE),
        (&a[0], t0 + LEASE + TimeDelta::seconds(1)),
    ] {
        let finished = dispatcher(&pool, refusing())
            .finish(claim, &defaults, at)
            .await
            .unwrap();
        assert!(matches!(finished, Finished::Failed(_)));
    }

    assert_eq!(
        warnings(&pool).await,
        [
            "Notification channel hooks gave up a Down notification for web on pve-01 after 2 \
          tries: target_not_permitted."
        ]
    );
    assert!(row(&pool, queued).await.given_up);
    assert_eq!(deliveries(&pool).await.len(), 2, "both sends are logged");
}

#[sqlx::test(migrations = "./migrations")]
async fn d_16_a_send_whose_outcome_is_never_written_ends_with_its_tries(pool: PgPool) {
    let (server, egress) = answering(NO_CONTENT).await;
    let hooks = channel(&pool, "hooks", webhook(&server)).await;
    let t0 = start(&pool).await;
    let queued = queue(&pool, hooks, None, t0).await;
    // The far future has no delivery partition, so every outcome write fails.
    let far = t0 + TimeDelta::days(400);
    let dispatcher = dispatcher(&pool, egress);

    let mut claimed = 0;
    for lease in 0..8 {
        claimed += dispatcher.tick(far + LEASE * lease).await;
    }

    assert_eq!(claimed, 6, "five tries, then one claim that gives up");
    server.settle().await;
    assert_eq!(server.connections(), 5);
    let state = row(&pool, queued).await;
    assert_eq!(
        (state.attempts, state.sent, state.given_up),
        (6, false, true)
    );
    assert_eq!(
        warnings(&pool).await,
        [
            "Notification channel hooks gave up a Down notification for web on pve-01 after 5 \
          tries: the outcome of the last try was not recorded."
        ]
    );
}
