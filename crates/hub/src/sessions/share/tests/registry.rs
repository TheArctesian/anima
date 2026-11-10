//! The share registry (S-1, S-2, S-3, S-5): the 32-share cap, which shares a session lists,
//! the owner rule of `endShare`, the first cause winning, and the keeper's take of a share.

use chrono::TimeDelta;
use tokio::sync::watch;
use uuid::Uuid;

use super::other_operator;
use crate::{
    ids::{SessionId, ShareId},
    sessions::{
        share::{
            Error,
            model::{ShareAccess, ShareEnd},
            registry::{MAX_SHARES, Share, Shares, settle},
        },
        tests::{
            fixture::{address, now, operator},
            tickets::session,
        },
        tickets::{TOKEN_BYTES, TicketHash, TicketToken},
    },
};

pub(super) fn share_id(n: u128) -> ShareId {
    ShareId::from(Uuid::from_u128(
        0x0190_a000_0000_7000_8020_0000_0000_0000 + n,
    ))
}

pub(super) fn hash(n: u8) -> TicketHash {
    TicketToken::from_random([n; TOKEN_BYTES]).hash()
}

/// Share `n` of `session`, made `n` seconds after `now()`.
pub(super) fn share(n: u8, session: SessionId) -> Share {
    let created_at = now() + TimeDelta::seconds(n.into());
    Share {
        id: share_id(n.into()),
        session,
        owner: operator(),
        access: ShareAccess::View,
        max_viewers: 3,
        viewers: 0,
        created_at,
        expires_at: created_at + TimeDelta::hours(1),
        end: watch::Sender::new(None),
    }
}

fn ids(shares: &Shares, of: SessionId) -> Vec<ShareId> {
    shares.of_session(of).iter().map(|share| share.id).collect()
}

#[test]
fn d_9_a_hub_holds_at_most_32_shares_ending_ones_included() {
    assert_eq!(MAX_SHARES, 32);
    let shares = Shares::default();
    for n in 0..32 {
        assert!(
            shares.open(hash(n), share(n, session(1))).is_ok(),
            "share {n}"
        );
    }
    assert!(shares.open(hash(32), share(32, session(2))).is_err());
    shares
        .end_by(share_id(0), &operator(), address("10.0.0.1"))
        .unwrap();
    assert!(
        shares.open(hash(32), share(32, session(2))).is_err(),
        "an ending share holds its place until its keeper removes it"
    );
    assert!(shares.remove(share_id(0)).is_some());
    assert!(shares.open(hash(32), share(32, session(2))).is_ok());
}

#[test]
fn ra_035_a_token_hash_opens_one_share_only() {
    let shares = Shares::default();
    assert!(shares.open(hash(1), share(1, session(1))).is_ok());
    assert!(shares.open(hash(1), share(2, session(1))).is_err());
    assert_eq!(ids(&shares, session(1)), [share_id(1)]);
}

#[test]
fn ra_035_a_session_lists_its_own_shares_that_no_cause_ended_oldest_first() {
    let shares = Shares::default();
    for (n, of) in [(3, 1), (1, 1), (2, 2), (4, 1)] {
        shares.open(hash(n), share(n, session(of))).unwrap();
    }
    assert_eq!(
        ids(&shares, session(1)),
        [share_id(1), share_id(3), share_id(4)]
    );
    assert_eq!(ids(&shares, session(2)), [share_id(2)]);
    assert_eq!(ids(&shares, session(9)), []);

    shares
        .end_by(share_id(3), &operator(), address("10.0.0.1"))
        .unwrap();
    assert_eq!(ids(&shares, session(1)), [share_id(1), share_id(4)]);
    assert_eq!(shares.hashes().len(), 4, "held until its keeper removes it");
}

#[test]
fn ra_035_only_the_owner_ends_a_share_and_only_once() {
    let shares = Shares::default();
    let mut ended = shares.open(hash(1), share(1, session(1))).unwrap();

    let refused = shares.end_by(share_id(1), &other_operator(), address("10.0.0.2"));
    assert!(matches!(refused, Err(Error::NotOwner)), "{refused:?}");
    assert_eq!(*ended.borrow_and_update(), None, "not signalled");
    assert_eq!(ids(&shares, session(1)), [share_id(1)]);

    let unknown = shares.end_by(share_id(9), &operator(), address("10.0.0.1"));
    assert!(matches!(unknown, Err(Error::UnknownShare(id)) if id == share_id(9)));

    shares
        .end_by(share_id(1), &operator(), address("10.0.0.1"))
        .unwrap();
    assert_eq!(
        *ended.borrow_and_update(),
        Some(ShareEnd::Ended(address("10.0.0.1"))),
        "the keeper is told who ended it from where"
    );
    let again = shares.end_by(share_id(1), &operator(), address("10.0.0.3"));
    assert!(matches!(again, Err(Error::UnknownShare(_))), "{again:?}");
    assert_eq!(
        *ended.borrow(),
        Some(ShareEnd::Ended(address("10.0.0.1"))),
        "unchanged"
    );
}

#[test]
fn ra_035_the_first_cause_wins() {
    let end = watch::Sender::new(None);
    assert_eq!(settle(&end, ShareEnd::Expired), ShareEnd::Expired);
    assert_eq!(settle(&end, ShareEnd::SessionEnded), ShareEnd::Expired);
    assert_eq!(*end.borrow(), Some(ShareEnd::Expired));
}

#[test]
fn ra_035_a_share_is_removed_once() {
    let shares = Shares::default();
    shares.open(hash(1), share(1, session(1))).unwrap();
    shares.open(hash(2), share(2, session(1))).unwrap();
    let removed = shares.remove(share_id(1)).unwrap();
    assert_eq!(removed.id, share_id(1));
    assert!(shares.remove(share_id(1)).is_none());
    assert!(shares.hashes() == [hash(2)], "the other share stays");
}

#[test]
fn s_5_a_cause_end_share_set_before_the_take_stands_over_the_one_the_keeper_woke_for() {
    let caller = address("10.0.0.9");
    for woke_for in [ShareEnd::Expired, ShareEnd::SessionEnded] {
        let shares = Shares::default();
        let mut ended = shares.open(hash(1), share(1, session(1))).unwrap();
        // The keeper woke for `woke_for`; endShare lands before it takes the share.
        shares.end_by(share_id(1), &operator(), caller).unwrap();

        let (taken, end) = shares.take(share_id(1), woke_for).unwrap();
        assert_eq!(taken.id, share_id(1));
        assert_eq!(end, ShareEnd::Ended(caller), "{woke_for:?}");
        assert_eq!(*ended.borrow_and_update(), Some(ShareEnd::Ended(caller)));
        assert!(shares.hashes().is_empty());
    }
}

#[test]
fn s_5_without_end_share_the_take_settles_the_cause_the_keeper_woke_for() {
    for woke_for in [ShareEnd::Expired, ShareEnd::SessionEnded] {
        let shares = Shares::default();
        let mut ended = shares.open(hash(1), share(1, session(1))).unwrap();

        let (_, end) = shares.take(share_id(1), woke_for).unwrap();
        assert_eq!(end, woke_for);
        assert_eq!(*ended.borrow_and_update(), Some(woke_for), "viewers see it");
    }
}

#[test]
fn s_5_a_taken_share_is_out_of_end_share_reach_and_taken_once() {
    let shares = Shares::default();
    shares.open(hash(1), share(1, session(1))).unwrap();
    let (taken, end) = shares.take(share_id(1), ShareEnd::Expired).unwrap();

    let late = shares.end_by(share_id(1), &operator(), address("10.0.0.9"));
    assert!(matches!(late, Err(Error::UnknownShare(_))), "{late:?}");
    assert_eq!(*taken.end.borrow(), Some(end), "the cause taken is final");
    assert!(shares.take(share_id(1), ShareEnd::SessionEnded).is_none());
}
