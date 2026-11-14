//! Decision 6: two live reads of an account at once; its changes one at a time, a second apart.

use std::{sync::Arc, time::Duration};

use chrono::TimeDelta;

use super::{account, now};
use crate::{
    clock::{Clock, FixedClock},
    engine::sources::live::pace::{Pace, gap},
};

/// Long enough for a waiting task to have run, short enough to keep the test quick.
const SETTLE: Duration = Duration::from_millis(50);

#[test]
fn prj_033_a_change_waits_until_a_second_after_the_previous_one_ended() {
    assert_eq!(gap(None, now()), Duration::ZERO);
    assert_eq!(gap(Some(now()), now()), Duration::from_secs(1));
    let later = now() + TimeDelta::milliseconds(400);
    assert_eq!(gap(Some(now()), later), Duration::from_millis(600));
    assert_eq!(
        gap(Some(now()), now() + TimeDelta::seconds(2)),
        Duration::ZERO
    );
}

#[tokio::test]
async fn prj_032_a_third_read_of_one_account_waits_and_another_account_does_not() {
    let pace = Pace::default();
    let id = account();
    let first = pace.read(id).await.unwrap();
    let second = pace.read(id).await.unwrap();
    let third = tokio::spawn({
        let pace = pace.clone();
        async move { pace.read(id).await.map(drop) }
    });

    tokio::time::sleep(SETTLE).await;
    assert!(!third.is_finished(), "two reads are in flight");
    let other = tokio::time::timeout(SETTLE, pace.read(account())).await;
    assert!(matches!(other, Ok(Ok(_))), "another account is not held up");

    drop(first);
    tokio::time::timeout(SETTLE, third)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop(second);
}

#[tokio::test(start_paused = true)]
async fn prj_033_changes_of_one_account_go_one_at_a_time_a_second_apart() {
    let pace = Pace::default();
    let id = account();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock(now()));
    let started = tokio::time::Instant::now();
    let turn = pace.change(id, clock.clone()).await.unwrap();
    assert_eq!(started.elapsed(), Duration::ZERO, "the first goes at once");
    let next = tokio::spawn({
        let (pace, clock) = (pace.clone(), clock.clone());
        async move { pace.change(id, clock).await.map(|_| started.elapsed()) }
    });

    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(!next.is_finished(), "the first change has not ended");
    let ended = started.elapsed();
    drop(turn);
    let waited = next.await.unwrap().unwrap();

    assert!(
        waited >= ended + Duration::from_secs(1),
        "{waited:?} after {ended:?}"
    );
    let other = pace.change(account(), clock).await;
    assert!(other.is_ok(), "another account is not held up");
    assert_eq!(started.elapsed(), waited, "and does not wait");
}
