//! The end signal of an open tunnel (D-10, K4 §5, LOG-005): `endSession` and shutdown reach
//! the tunnel with the cause and who asked, and a redemption never slips past `endSession`.

use crate::fleet::sessions::Presented;
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use chrono::TimeDelta;

use super::tickets::{CLIENT, client, issue, issued_at, pending, session, token};
use crate::{
    fleet::sessions::{
        Actor, EndCause, EndReason, Protocol, Session,
        live::{Ending, Live, Refused, Unavailable},
    },
    ids::SessionId,
};

fn operator_actor() -> Actor {
    Actor {
        operator: pending(1).operator,
        client: client("10.10.0.80"),
        at: issued_at() + TimeDelta::seconds(3),
    }
}

#[tokio::test]
async fn ra_008_end_by_signals_operator_with_who_asked_and_finishes_when_the_tunnel_drops() {
    let live = Live::default();
    let text = issue(&live, 1, 4);
    let mut tunnel = live
        .redeem(Presented::terminal(&text, client(CLIENT)), issued_at())
        .unwrap()
        .tunnel;

    let Ending::Signalled(closing) = live.end_by(session(1), &operator_actor()) else {
        panic!("the tunnel is registered");
    };
    let expected = EndCause {
        reason: EndReason::Operator,
        by: Some(operator_actor()),
    };
    assert_eq!(tunnel.ended().await, expected);
    let finished = tokio::spawn(closing.finished());
    tokio::task::yield_now().await;
    assert!(!finished.is_finished(), "waits for the tunnel to drop");
    drop(tunnel);
    tokio::time::timeout(Duration::from_secs(5), finished)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(live.end_by(session(1), &operator_actor()), Ending::Neither),
        "dropping deregisters"
    );
}

#[test]
fn ra_008_end_by_withdraws_a_pending_or_spent_ticket() {
    let live = Live::default();
    issue(&live, 1, 1);
    let spent = issue(&live, 2, 2);
    assert!(
        live.redeem(
            Presented::terminal(&spent, client("10.10.0.99")),
            issued_at()
        )
        .is_err()
    );
    for n in [1, 2] {
        let Ending::Withdrawn(ticket) = live.end_by(session(n), &operator_actor()) else {
            panic!("ticket {n} is pending or spent");
        };
        assert_eq!(ticket.session, session(n));
        assert!(matches!(
            live.end_by(session(n), &operator_actor()),
            Ending::Neither
        ));
    }
    assert_eq!(live.take_expired(pending(1).expires_at).len(), 0);
}

/// D-10 regression: `endSession` and a redemption racing on real threads. Under one lock the
/// outcome is always consistent: the tunnel opened and was told to end, or the ticket was
/// withdrawn and the redemption refused. Never an open tunnel nobody signalled.
#[test]
fn ra_008_end_by_and_redeem_never_both_miss_on_real_threads() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    for round in 0..20_000_usize {
        let live = Live::default();
        let text = issue(&live, 1, u8::try_from(round % 251).unwrap());
        let barrier = Arc::new(Barrier::new(2));
        let (redeemed, ending) = thread::scope(|scope| {
            let redeemer = scope.spawn(|| {
                barrier.wait();
                live.redeem(Presented::terminal(&text, client(CLIENT)), issued_at())
            });
            let ender = scope.spawn(|| {
                barrier.wait();
                live.end_by(session(1), &operator_actor())
            });
            (redeemer.join().unwrap(), ender.join().unwrap())
        });
        match (redeemed, ending) {
            (Ok(mut redeemed), Ending::Signalled(_)) => {
                let cause = runtime.block_on(redeemed.tunnel.ended());
                assert_eq!(cause.reason, EndReason::Operator, "round {round}");
            }
            (Err(Refused), Ending::Withdrawn(_)) => {}
            (redeemed, _) => panic!("round {round}: inconsistent, redeemed {}", redeemed.is_ok()),
        }
    }
}

#[tokio::test]
async fn ra_008_shut_down_signals_every_tunnel_withdraws_every_ticket_and_refuses_new_ones() {
    let live = Live::default();
    let first = issue(&live, 1, 1);
    let second = issue(&live, 2, 2);
    let pending_third = issue(&live, 3, 3);
    let mut one = live
        .redeem(Presented::terminal(&first, client(CLIENT)), issued_at())
        .unwrap()
        .tunnel;
    let mut two = live
        .redeem(Presented::terminal(&second, client(CLIENT)), issued_at())
        .unwrap()
        .tunnel;

    let shutdown = EndCause {
        reason: EndReason::HubShutdown,
        by: None,
    };
    let (closing, withdrawn) = live.shut_down();
    let withdrawn: Vec<SessionId> = withdrawn.iter().map(|ticket| ticket.session).collect();
    assert_eq!(
        withdrawn,
        [session(3)],
        "the unredeemed ticket is handed back to end its row"
    );
    assert!(live.withdraw(session(3)).is_none(), "and no longer held");
    assert_eq!(one.ended().await, shutdown);
    assert_eq!(two.ended().await, shutdown);
    // The first cause wins: a later endSession does not change it.
    drop(live.end_by(session(1), &operator_actor()));
    assert_eq!(one.ended().await, shutdown);
    assert_eq!(
        live.redeem(
            Presented::terminal(&pending_third, client(CLIENT)),
            issued_at()
        )
        .err(),
        Some(Refused)
    );
    assert_eq!(live.issue(token(9).hash(), pending(9)), Err(Unavailable));
    drop((one, two));
    tokio::time::timeout(Duration::from_secs(5), closing.finished())
        .await
        .unwrap();
}

#[test]
fn k4_1_a_ticket_opens_only_on_its_tunnel_path() {
    assert_eq!(Protocol::Ssh.tunnel_path(), "/terminal");
    let live = Live::default();
    let text = issue(&live, 1, 4);
    let presented = Presented::terminal(&text, client(CLIENT));
    let _redeemed = live.redeem(presented, issued_at()).unwrap();
    assert!(
        matches!(
            live.end_by(session(1), &operator_actor()),
            Ending::Signalled(_)
        ),
        "its tunnel is registered"
    );
}

#[test]
fn k4_1_a_ticket_on_another_tunnel_path_is_refused_and_spent_in_the_redemption() {
    let live = Live::default();
    let text = issue(&live, 1, 4);
    let on_tunnel = Presented {
        path: "/tunnel",
        ..Presented::terminal(&text, client(CLIENT))
    };
    assert_eq!(live.redeem(on_tunnel, issued_at()).err(), Some(Refused));
    let presented = Presented::terminal(&text, client(CLIENT));
    assert_eq!(
        live.redeem(presented, issued_at()).err(),
        Some(Refused),
        "spent"
    );
    // No tunnel was registered in between, so an `endSession` can never signal one that is
    // then dropped: it withdraws the spent ticket and ends the row itself.
    assert!(
        matches!(
            live.end_by(session(1), &operator_actor()),
            Ending::Withdrawn(_)
        ),
        "the ticket waits for endSession or the sweep"
    );
}

/// The committed row of ticket `n`'s session, as `start` writes it.
fn started_row(n: u128) -> Session {
    let ticket = pending(n);
    Session {
        id: ticket.session,
        device: Some(ticket.device),
        device_name: "docker-host".to_owned(),
        protocol: ticket.protocol,
        operator: ticket.operator.id,
        operator_name: ticket.operator.name,
        client: ticket.client.0,
        started_at: issued_at(),
        end: None,
    }
}

#[test]
fn ra_034_a_start_whose_ticket_end_session_withdrew_is_not_published() {
    let live = Live::default();
    let mut changes = live.changes();
    issue(&live, 1, 1);
    issue(&live, 2, 2);
    // `endSession` withdraws ticket 1 after its row commits and before the start publishes.
    assert!(matches!(
        live.end_by(session(1), &operator_actor()),
        Ending::Withdrawn(_)
    ));

    assert!(live.publish_start(started_row(1)), "not shutting down");
    assert!(live.publish_start(started_row(2)));

    let change = changes.try_recv().expect("the held ticket's start");
    assert_eq!(change.session.id, session(2));
    assert!(
        changes.try_recv().is_err(),
        "no start for the withdrawn ticket"
    );
}
