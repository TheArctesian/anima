//! The keeper's timing and its record (RA-035, D-9, S-5): a share ends at its expiry and not
//! before, on the tokio clock; `share.end` records the cause that stands and where it came from.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::watch;
use uuid::Uuid;

use super::{CLIENT, bench, ending::sharing, share_end};
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    ids::{SessionId, ShareId},
    sessions::{
        EndCause, Live,
        share::{
            Sharing,
            keeper::{Keeper, recorded_from},
            model::{ShareAccess, ShareEnd},
            registry::Share,
        },
        tests::fixture::{address, now, operator},
        tickets::TicketToken,
    },
};

#[test]
fn ra_035_end_share_records_its_caller_and_any_other_cause_where_the_owner_shared_from() {
    let owner = address(CLIENT);
    let caller = address("100.64.0.9");
    assert_eq!(recorded_from(ShareEnd::Ended(caller), owner), caller);
    for end in [ShareEnd::Expired, ShareEnd::SessionEnded] {
        assert_eq!(recorded_from(end, owner), owner, "{end:?}");
    }
}

/// The keeper of `share`, held under `token` in `sharing`'s registry, whose owner shared from
/// [`CLIENT`] and whose tunnel signals on `tunnel_end`.
fn keeper_of(
    sharing: Sharing,
    share: Share,
    token: &TicketToken,
    tunnel_end: watch::Receiver<Option<EndCause>>,
) -> Keeper {
    let (id, expires_at) = (share.id, share.expires_at);
    let ended = sharing.live.shares().open(token.hash(), share).unwrap();
    Keeper {
        sharing,
        id,
        expires_at,
        client: address(CLIENT),
        device: None,
        ended,
        tunnel_end,
    }
}

/// A share of `session` that expires `lifetime` after `now()`.
fn share_for(session: SessionId, lifetime: TimeDelta) -> Share {
    Share {
        id: ShareId::from(Uuid::now_v7()),
        session,
        owner: operator(),
        access: ShareAccess::View,
        max_viewers: 3,
        viewers: 0,
        created_at: now(),
        expires_at: now() + lifetime,
        end: watch::Sender::new(None),
    }
}

/// S-5: the keeper woke for expiry or the session's end, then `endShare` landed before it
/// removed the share. `share.end` records `endShare`'s cause and caller, as viewers see it.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_5_end_share_set_after_the_keeper_woke_is_the_cause_share_end_records(pool: PgPool) {
    let bench = bench(&pool).await;
    let caller = address("100.64.0.9");
    for (n, woke_for) in [(1, ShareEnd::Expired), (2, ShareEnd::SessionEnded)] {
        let share = share_for(bench.session, TimeDelta::minutes(10));
        let (id, mut seen) = (share.id, share.end.subscribe());
        let token = TicketToken::from_random([n; 32]);
        let keeper = keeper_of(sharing(&bench), share, &token, watch::channel(None).1);
        bench.live.shares().end_by(id, &operator(), caller).unwrap();

        keeper.end(woke_for).await;
        let (client, change) = share_end(&pool, &id.to_string()).await;
        assert_eq!(change["cause"], "ended", "{woke_for:?}");
        assert_eq!(client, "100.64.0.9");
        assert_eq!(*seen.borrow_and_update(), Some(ShareEnd::Ended(caller)));
    }
    assert!(bench.live.shares().hashes().is_empty());
}

/// Without `endShare`, `share.end` records the cause the keeper woke for, from the owner.
#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn s_5_without_end_share_share_end_records_the_cause_the_keeper_woke_for(pool: PgPool) {
    let bench = bench(&pool).await;
    for (n, woke_for, cause) in [
        (1, ShareEnd::Expired, "expired"),
        (2, ShareEnd::SessionEnded, "session_ended"),
    ] {
        let share = share_for(bench.session, TimeDelta::minutes(10));
        let id = share.id;
        let token = TicketToken::from_random([n; 32]);
        let keeper = keeper_of(sharing(&bench), share, &token, watch::channel(None).1);

        keeper.end(woke_for).await;
        let (client, change) = share_end(&pool, &id.to_string()).await;
        assert_eq!(change["cause"], cause);
        assert_eq!(client, CLIENT);
    }
}

/// Lets every ready task run.
async fn settle_tasks() {
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
}

/// D-9: the keeper sleeps exactly until `expires_at` on the hub's clock. The database never
/// answers, so only the wait is observed: the share is held until then and ended at once.
#[tokio::test(start_paused = true)]
async fn d_9_a_share_ends_at_its_expiry_and_not_a_millisecond_before() {
    let lifetime = TimeDelta::minutes(10);
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let sharing = Sharing {
        db: PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(300))
            .connect_lazy("postgres://anima@127.0.0.1:9/never")
            .unwrap(),
        bus: Bus::new(16),
        clock,
        live: Live::default(),
    };
    let share = share_for(Uuid::now_v7().into(), lifetime);
    let mut seen = share.end.subscribe();
    let token = TicketToken::from_random([7; 32]);
    let shares = sharing.live.shares().clone();
    let (_tunnel, tunnel_end) = watch::channel(None);
    let keeper = keeper_of(sharing, share, &token, tunnel_end);
    tokio::spawn(keeper.keep());
    settle_tasks().await;

    let almost = (lifetime - TimeDelta::milliseconds(1)).to_std().unwrap();
    tokio::time::advance(almost).await;
    settle_tasks().await;
    assert!(shares.hashes() == [token.hash()], "held until its expiry");
    assert_eq!(*seen.borrow_and_update(), None);

    tokio::time::advance(Duration::from_millis(1)).await;
    settle_tasks().await;
    assert!(shares.hashes().is_empty(), "removed at its expiry");
    assert_eq!(*seen.borrow_and_update(), Some(ShareEnd::Expired));
}
