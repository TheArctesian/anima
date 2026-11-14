//! PRJ-039: how an attempt ends (expiry, the store, a cancel, a failed exchange), with the time
//! as a parameter. Every end drops the key.

use super::{app, app_created, at, at_github, exchanging, key, phase, t0, value, waiting};
use crate::{
    engine::sources::registration::attempt::{Failure, Finished, Phase, Refusal},
    ids::SourceAccountId,
};

#[test]
fn prj_039_each_open_phase_expires_at_55_minutes_and_drops_the_key() {
    let walks = [
        ("waiting", waiting()),
        ("at github", at_github().attempt),
        ("exchanging", exchanging().attempt),
        ("app created", app_created().attempt),
    ];
    for (name, mut attempt) in walks {
        let before = phase(&attempt);
        assert!(!attempt.expire(at(54, 59)), "{name}");
        assert_eq!(phase(&attempt), before, "{name}");
        assert!(attempt.expire(at(55, 0)), "{name}");
        let shown = attempt.snapshot();
        assert_eq!(
            (shown.phase, shown.failure),
            (Phase::Failed, Some(Failure::Expired)),
            "{name}"
        );
        assert!(attempt.waiting_app().is_none(), "{name}: no key");
        assert!(!attempt.expire(at(56, 0)), "{name}: once");
    }
    let mut walked = app_created();
    let round = walked.attempt.begin_finish(at(54, 0)).unwrap().round;
    assert!(!walked.attempt.expire(at(55, 30)), "the store decides");
    walked.attempt.finished(round, Finished::Again, at(55, 30));
    assert_eq!(walked.attempt.snapshot().failure, Some(Failure::Expired));
}

#[test]
fn prj_039_only_one_caller_stores_and_a_failed_store_waits_for_the_next_poll() {
    let mut walked = app_created();
    let round = walked.attempt.begin_finish(t0()).unwrap().round;
    assert!(matches!(
        walked.attempt.begin_finish(t0()),
        Err(Refusal::Conflict)
    ));
    assert!(
        walked.attempt.waiting_app().is_none(),
        "no poll while storing"
    );
    walked.attempt.finished(round, Finished::Again, at(1, 0));
    assert!(walked.attempt.waiting_app().is_some(), "the key is back");
    let round = walked.attempt.begin_finish(at(1, 0)).unwrap().round;
    walked
        .attempt
        .finished(round, Finished::Failed(Failure::KeyRefused), at(2, 0));
    let shown = walked.attempt.snapshot();
    assert_eq!(
        (shown.phase, shown.failure),
        (Phase::Failed, Some(Failure::KeyRefused))
    );
    assert!(walked.attempt.waiting_app().is_none());
}

#[test]
fn prj_039_a_store_that_holds_finishing_past_two_minutes_goes_back_to_app_created() {
    let mut walked = app_created();
    let first = walked.attempt.begin_finish(at(10, 0)).unwrap().round;
    assert!(!walked.attempt.expire(at(11, 59)));
    assert!(walked.attempt.waiting_app().is_none(), "still storing");
    assert!(!walked.attempt.expire(at(12, 0)));
    assert!(walked.attempt.waiting_app().is_some(), "back with its key");
    let second = walked.attempt.begin_finish(at(12, 0)).unwrap().round;
    for late in [Finished::Again, Finished::Failed(Failure::KeyRefused)] {
        walked.attempt.finished(first, late, at(12, 30));
        assert!(
            walked.attempt.waiting_app().is_none() && !walked.attempt.ended(),
            "{late:?} of the first store changes nothing"
        );
    }
    let account = SourceAccountId::from(uuid::Uuid::now_v7());
    let connected = Finished::Connected {
        account,
        consumer_on: false,
    };
    walked.attempt.finished(first, connected, at(12, 40));
    assert_eq!(
        phase(&walked.attempt),
        Phase::Connected,
        "a late success counts"
    );
    walked.attempt.finished(second, Finished::Again, at(12, 50));
    assert_eq!(phase(&walked.attempt), Phase::Connected);

    let mut late = app_created();
    late.attempt.begin_finish(at(54, 0)).unwrap();
    assert!(!late.attempt.expire(at(55, 30)), "the store decides");
    assert!(
        late.attempt.expire(at(56, 0)),
        "past the limit, expiry applies"
    );
    assert_eq!(late.attempt.snapshot().failure, Some(Failure::Expired));
}

#[test]
fn prj_039_cancel_drops_the_key_and_is_refused_once_connected() {
    for mut walked in [at_github(), exchanging(), app_created()] {
        assert_eq!(walked.attempt.cancel(at(1, 0)), Ok(()));
        assert_eq!(phase(&walked.attempt), Phase::Cancelled);
        assert!(walked.attempt.waiting_app().is_none());
        assert_eq!(walked.attempt.cancel(at(2, 0)), Ok(()), "again: no change");
    }
    let mut storing = app_created();
    let round = storing.attempt.begin_finish(at(1, 0)).unwrap().round;
    assert_eq!(storing.attempt.cancel(at(1, 0)), Err(Refusal::Conflict));
    let account = SourceAccountId::from(uuid::Uuid::now_v7());
    let connected = Finished::Connected {
        account,
        consumer_on: false,
    };
    storing.attempt.finished(round, connected, at(1, 0));
    assert_eq!(storing.attempt.cancel(at(2, 0)), Err(Refusal::Conflict));
}

#[test]
fn prj_039_an_app_made_after_a_cancel_is_named_but_its_key_is_dropped() {
    let mut walked = exchanging();
    walked.attempt.cancel(at(1, 0)).unwrap();
    assert_eq!(
        walked.attempt.created(app(), key(), value().hash()),
        Err(Refusal::Conflict)
    );
    let shown = walked.attempt.snapshot();
    assert_eq!((shown.phase, shown.app), (Phase::Cancelled, Some(app())));
    assert!(walked.attempt.waiting_app().is_none());
}

#[test]
fn prj_039_a_failed_exchange_ends_the_attempt_with_its_failure() {
    let mut walked = exchanging();
    walked
        .attempt
        .exchange_failed(Failure::ExchangeRefused, None, at(1, 0));
    let shown = walked.attempt.snapshot();
    assert_eq!(
        (shown.phase, shown.failure),
        (Phase::Failed, Some(Failure::ExchangeRefused))
    );
    assert!(walked.attempt.readable(at(10, 59)));
    assert!(
        !walked.attempt.readable(at(11, 0)),
        "ten minutes after the end"
    );
}
