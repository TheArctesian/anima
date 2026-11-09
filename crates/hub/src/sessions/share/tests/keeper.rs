//! The keeper's timing and its record (RA-035, D-9, S-5): a share ends at its expiry and not
//! before, on the tokio clock; `share.end` records the cause that stands and where it came from.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch;
use uuid::Uuid;

use super::CLIENT;
use crate::{
    clock::{Clock, FixedClock},
    events::Bus,
    ids::ShareId,
    sessions::{
        Live,
        share::{
            Sharing,
            keeper::{Keeper, recorded},
            model::{ShareAccess, ShareEnd},
            registry::Share,
        },
        tests::fixture::{address, now, operator},
        tickets::TicketToken,
    },
};

#[test]
fn ra_035_a_cause_end_share_set_first_stands_over_the_one_the_keeper_woke_for() {
    let owner = address(CLIENT);
    let caller = address("100.64.0.9");
    for woke_for in [ShareEnd::Expired, ShareEnd::SessionEnded] {
        assert_eq!(
            recorded(Some(ShareEnd::Ended(caller)), woke_for, owner),
            (ShareEnd::Ended(caller), caller),
            "{woke_for:?}"
        );
    }
}

#[test]
fn ra_035_without_end_share_the_cause_the_keeper_woke_for_is_recorded_from_the_owner() {
    let owner = address(CLIENT);
    let caller = address("100.64.0.9");
    for woke_for in [ShareEnd::Expired, ShareEnd::SessionEnded] {
        assert_eq!(recorded(None, woke_for, owner), (woke_for, owner));
    }
    assert_eq!(
        recorded(None, ShareEnd::Ended(caller), owner),
        (ShareEnd::Ended(caller), caller)
    );
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
    let share = Share {
        id: ShareId::from(Uuid::now_v7()),
        session: Uuid::now_v7().into(),
        owner: operator(),
        access: ShareAccess::View,
        max_viewers: 3,
        viewers: 0,
        created_at: now(),
        expires_at: now() + lifetime,
        end: watch::Sender::new(None),
    };
    let (id, expires_at) = (share.id, share.expires_at);
    let mut seen = share.end.subscribe();
    let token = TicketToken::from_random([7; 32]);
    let shares = sharing.live.shares().clone();
    let ended = shares.open(token.hash(), share).unwrap();
    let (_tunnel, tunnel_end) = watch::channel(None);
    let keeper = Keeper {
        sharing,
        id,
        expires_at,
        client: address(CLIENT),
        device: None,
        ended,
        tunnel_end,
    };
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
