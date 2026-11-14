//! PRJ-039, NET-002: the watch's pace (backoff while GitHub does not answer, no wake sooner
//! than one wait after the last ask) and the store that is dropped or panics, with the time as
//! a parameter.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;
use uuid::Uuid;

use super::{app_created, at, t0};
use crate::{
    clock::{Clock, FixedClock},
    engine::sources::registration::{
        attempt::{Finished, Phase},
        attempts::{Attempts, Settle},
        installation::{BACKOFF_CAP, Pace},
    },
    ids::SourceAccountId,
};

fn wait(pace: &Pace) -> TimeDelta {
    TimeDelta::from_std(pace.wait()).unwrap()
}

#[test]
fn net_002_the_watch_backs_off_while_github_does_not_answer_and_a_success_resets_it() {
    let mut pace = Pace::new(Duration::from_secs(5));
    assert!(pace.woken_may_ask(t0()), "no ask yet: a wake asks");
    // (GitHub answered, the wait after it in seconds)
    let table: [(bool, i64); 9] = [
        (true, 5),
        (false, 10),
        (false, 20),
        (false, 40),
        (false, 80),
        (false, 160),
        (false, 300),
        (false, 300),
        (true, 5),
    ];
    let mut now = t0();
    for (answered, seconds) in table {
        pace.asked(now, answered);
        let expected = TimeDelta::seconds(seconds);
        assert_eq!(wait(&pace), expected, "{answered} {seconds}");
        assert!(!pace.woken_may_ask(now), "{seconds}: a wake at once");
        assert!(
            !pace.woken_may_ask(now + expected - TimeDelta::seconds(1)),
            "{seconds}: a wake within the wait"
        );
        assert!(
            pace.woken_may_ask(now + expected),
            "{seconds}: a wake after it"
        );
        now += expected;
    }
    for _ in 0..200 {
        pace.asked(now, false);
    }
    assert_eq!(pace.wait(), BACKOFF_CAP, "never past the cap");
    let slow = Pace::new(Duration::from_secs(3600));
    assert_eq!(
        slow.wait(),
        Duration::from_secs(3600),
        "a slower poll is kept"
    );
}

fn storing(attempts: &Attempts, id: Uuid) -> u32 {
    attempts
        .step(Some(id), at(1, 0), |held| held.begin_finish(at(1, 0)))
        .unwrap()
        .round
}

fn waits_for_install(attempts: &Attempts, id: Uuid) -> bool {
    attempts
        .step(Some(id), at(1, 0), |held| Ok(held.waiting_app().is_some()))
        .unwrap()
}

/// What a store holds while it runs: back to `APP_CREATED` unless told otherwise.
fn settle(attempts: &Attempts, id: Uuid, round: u32) -> Settle {
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(at(1, 0)));
    Settle::new(attempts.clone(), id, clock, move |held, now| {
        held.finished(round, Finished::Again, now);
    })
}

#[tokio::test]
async fn prj_039_a_store_that_is_dropped_or_panics_leaves_app_created_and_a_later_store_finishes() {
    let attempts = Attempts::new(Duration::from_secs(5));
    let id = attempts.start(app_created().attempt, t0()).unwrap().id;

    let round = storing(&attempts, id);
    assert!(!waits_for_install(&attempts, id), "storing");
    drop(settle(&attempts, id, round));
    assert!(
        waits_for_install(&attempts, id),
        "dropped: back to APP_CREATED"
    );

    let moved = attempts.clone();
    let task = tokio::spawn(async move {
        let _settle = settle(&moved, id, storing(&moved, id));
        panic!("the store failed");
    });
    assert!(task.await.unwrap_err().is_panic());
    assert!(
        waits_for_install(&attempts, id),
        "panicked: back to APP_CREATED"
    );

    let round = storing(&attempts, id);
    let mut finished = settle(&attempts, id, round);
    let account = SourceAccountId::from(Uuid::now_v7());
    finished.instead(move |held, now| {
        let connected = Finished::Connected {
            account,
            consumer_on: false,
        };
        held.finished(round, connected, now);
    });
    drop(finished);
    let shown = attempts.current(at(1, 0)).unwrap();
    assert_eq!(
        (shown.phase, shown.account),
        (Phase::Connected, Some(account))
    );
}
