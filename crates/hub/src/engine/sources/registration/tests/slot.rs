//! PRJ-039: the one attempt the hub holds: a new start replaces one with no App, is refused
//! while an App waits for Install, and each change is published once.

use std::time::Duration;

use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

use super::{app_created, at, at_github, request, t0, waiting};
use crate::engine::sources::registration::{
    attempt::{Attempt, Failure, Phase, Refusal},
    attempts::Attempts,
};

fn slot() -> Attempts {
    Attempts::new(Duration::from_secs(5))
}

#[test]
fn prj_039_a_start_replaces_an_attempt_that_made_no_app() {
    let attempts = slot();
    let first = attempts.start(waiting(), t0()).unwrap();
    let second = attempts.start(at_github().attempt, at(1, 0)).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(attempts.current(at(1, 0)).map(|s| s.id), Some(second.id));
    assert_eq!(
        attempts.step(Some(first.id), at(1, 0), |held| held.cancel(at(1, 0))),
        Err(Refusal::NotFound),
        "the replaced attempt is gone"
    );
}

#[test]
fn prj_039_a_start_is_refused_while_an_app_waits_for_install() {
    let attempts = slot();
    let waiting_app = attempts.start(app_created().attempt, t0()).unwrap();
    let refused = attempts.start(waiting(), at(1, 0));
    assert_eq!(refused.err(), Some(Refusal::AppWaiting));
    assert_eq!(
        attempts.current(at(1, 0)).map(|s| (s.id, s.phase)),
        Some((waiting_app.id, Phase::AppCreated))
    );
    attempts
        .step(None, at(2, 0), |held| held.cancel(at(2, 0)))
        .unwrap();
    assert!(
        attempts.start(waiting(), at(3, 0)).is_ok(),
        "after the cancel"
    );
}

#[test]
fn prj_039_an_expired_app_no_longer_blocks_a_start() {
    let attempts = slot();
    attempts.start(app_created().attempt, t0()).unwrap();
    let shown = attempts.current(at(55, 0)).unwrap();
    assert_eq!(shown.failure, Some(Failure::Expired));
    assert!(attempts.start(waiting(), at(55, 0)).is_ok());
}

#[test]
fn prj_039_an_ended_attempt_is_readable_for_ten_minutes() {
    let attempts = slot();
    let started = attempts.start(waiting(), t0()).unwrap();
    attempts
        .step(Some(started.id), at(1, 0), |held| held.cancel(at(1, 0)))
        .unwrap();
    assert_eq!(
        attempts.current(at(10, 59)).map(|s| s.phase),
        Some(Phase::Cancelled)
    );
    assert!(attempts.current(at(11, 0)).is_none());
    assert_eq!(
        attempts.step(Some(started.id), at(11, 0), |held| held.cancel(at(11, 0))),
        Err(Refusal::NotFound)
    );
}

#[test]
fn prj_039_each_change_is_published_once_and_a_read_publishes_nothing() {
    let attempts = slot();
    let mut received = attempts.subscribe();
    let started = attempts.start(waiting(), t0()).unwrap();
    assert_eq!(received.try_recv().map(|s| s.id), Ok(started.id));
    let _ = attempts.current(at(1, 0));
    assert_eq!(received.try_recv().err(), Some(TryRecvError::Empty));
    let refused = attempts.step(None, at(1, 0), |held| {
        held.claim_ticket("x", super::value().hash(), at(1, 0))
    });
    assert_eq!(refused, Err(Refusal::Ticket));
    assert_eq!(
        received.try_recv().err(),
        Some(TryRecvError::Empty),
        "a refusal changes nothing"
    );
    let _ = attempts.current(at(55, 0));
    let expired = received.try_recv().unwrap();
    assert_eq!(
        expired.failure,
        Some(Failure::Expired),
        "the expiry is a change"
    );
    assert_eq!(received.try_recv().err(), Some(TryRecvError::Empty));
}

#[test]
fn prj_039_forget_drops_only_its_own_attempt() {
    let attempts = slot();
    let kept = attempts.start(waiting(), t0()).unwrap();
    attempts.forget(Uuid::now_v7(), t0());
    assert_eq!(attempts.current(t0()).map(|s| s.id), Some(kept.id));
    attempts.forget(kept.id, t0());
    assert!(attempts.current(t0()).is_none());
    let label = Attempt::start(Uuid::now_v7(), request(Some("Mine")), t0());
    assert_eq!(label.request().label.as_deref(), Some("Mine"));
}
