//! A stop request while the group starts, and a migration that is given up.

use std::time::Duration;

use super::{Rig, never_asked, rig};
use crate::child::Stop;
use crate::run::Ended;
use crate::state::State;

#[tokio::test]
async fn a_stop_that_was_asked_before_the_start_starts_nothing() {
    let rig = rig();
    let (ask, stop) = Stop::new();
    ask.send_replace(true);

    let ended = rig
        .run_with(stop, || panic!("it must not get ready"))
        .await
        .unwrap();

    assert_eq!(ended, Ended::Asked);
    assert_eq!(rig.events(), Vec::<String>::new());
    assert_eq!(rig.state(), Some(State::Stopped));
}

/// Runs the group, asks it to stop when `event` is in the events, and tells how long the
/// run took from there.
async fn run_until_event(rig: &Rig, event: &str) -> (Ended, Duration) {
    let (ask, stop) = Stop::new();
    let asking = async {
        while !rig.events().iter().any(|line| line == event) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ask.send_replace(true);
        std::time::Instant::now()
    };
    let (ended, asked_at) = tokio::join!(rig.run_with(stop, || panic!("not ready")), asking);
    (ended.unwrap(), asked_at.elapsed())
}

#[tokio::test]
async fn a_stop_during_a_long_database_start_does_not_wait_for_the_start() {
    let rig = rig();
    rig.set("database-slow", "");

    let (ended, took) = run_until_event(&rig, "database start").await;

    assert_eq!(ended, Ended::Asked);
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert_eq!(rig.events(), ["database start", "database stop"]);
    assert_eq!(rig.state(), Some(State::Stopped));
}

#[tokio::test]
async fn a_stop_during_the_migration_ends_the_migration_and_then_the_database() {
    let rig = rig();
    rig.set("migrate-slow", "");

    let (ended, took) = run_until_event(&rig, "migrate as owner.url").await;

    assert_eq!(ended, Ended::Asked);
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert_eq!(
        rig.events()[1..],
        ["migrate as owner.url", "migrate stop", "database stop"]
    );
}

#[tokio::test]
async fn a_migration_that_runs_past_its_limit_is_stopped_and_the_run_fails() {
    let mut rig = rig();
    rig.set("migrate-slow", "");
    rig.plan.limits.migration = Duration::from_millis(300);
    let (_ask, stop) = never_asked();

    let ended = rig.run_with(stop, || {}).await.unwrap();

    assert_eq!(ended, Ended::Failed);
    assert_eq!(
        rig.events()[1..],
        ["migrate as owner.url", "migrate stop", "database stop"]
    );
}
