use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, TimeZone as _, Utc};

use super::{next_daily_run, plan, retry_delay, run_daily};
use crate::clock::Clock;

fn at(day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2030, 5, day, hour, minute, second)
        .unwrap()
}

const MINUTE: Duration = Duration::from_secs(60);

#[test]
fn db_004_the_daily_run_is_the_next_ten_past_midnight_utc() {
    assert_eq!(next_daily_run(at(5, 0, 9, 59)), at(5, 0, 10, 0));
    assert_eq!(next_daily_run(at(5, 0, 10, 0)), at(6, 0, 10, 0));
    assert_eq!(next_daily_run(at(5, 12, 0, 0)), at(6, 0, 10, 0));
    assert_eq!(next_daily_run(at(5, 23, 59, 59)), at(6, 0, 10, 0));
}

#[test]
fn db_004_retries_double_from_thirty_seconds_up_to_thirty_minutes_eight_times() {
    let delays: Vec<_> = (0..=9).map(retry_delay).collect();
    let seconds = |s: u64| Some(Duration::from_secs(s));
    assert_eq!(
        delays,
        [
            None,
            seconds(30),
            seconds(60),
            seconds(120),
            seconds(240),
            seconds(480),
            seconds(960),
            seconds(1800),
            seconds(1800),
            None,
        ]
    );
    assert_eq!(retry_delay(u32::MAX), None);
}

#[test]
fn db_004_the_plan_waits_for_the_daily_run_after_success_or_spent_retries() {
    let noon = at(5, 12, 0, 0);
    let until_daily = Duration::from_mins(12 * 60 + 10);
    assert_eq!(plan(noon, 0), (until_daily, 0));
    assert_eq!(plan(noon, 1), (Duration::from_secs(30), 1));
    assert_eq!(plan(noon, 9), (until_daily, 0));
}

#[test]
fn db_004_a_retry_never_waits_past_the_daily_run() {
    let just_before = at(5, 0, 9, 0);
    assert_eq!(plan(just_before, 3), (MINUTE, 3));
    assert_eq!(plan(at(5, 0, 10, 0), 0).0, Duration::from_hours(24));
}

/// A clock that follows tokio's (paused) time from `base`.
struct TokioClock {
    base: DateTime<Utc>,
    start: tokio::time::Instant,
}

impl Clock for TokioClock {
    fn now(&self) -> DateTime<Utc> {
        self.base + TimeDelta::from_std(self.start.elapsed()).unwrap()
    }
}

#[tokio::test(start_paused = true)]
async fn db_004_a_job_runs_at_start_retries_after_failures_then_runs_daily() {
    let clock: Arc<dyn Clock> = Arc::new(TokioClock {
        base: at(5, 12, 0, 0),
        start: tokio::time::Instant::now(),
    });
    let outcomes = Arc::new(Mutex::new(VecDeque::from([
        Err("lock_timeout"),
        Err("database"),
        Ok(()),
        Ok(()),
    ])));
    let runs = Arc::new(Mutex::new(Vec::new()));
    let job = {
        let (outcomes, runs) = (outcomes.clone(), runs.clone());
        move |now: DateTime<Utc>| {
            let mut runs = runs.lock().unwrap();
            runs.push(now);
            // A plan that never waits would spin here; stop it instead of hanging the test.
            assert!(runs.len() <= 10, "the job runs without waiting");
            drop(runs);
            let outcome = outcomes.lock().unwrap().pop_front().unwrap_or(Ok(()));
            async move { outcome }
        }
    };
    let task = tokio::spawn(run_daily("test", clock, job));
    tokio::time::sleep(Duration::from_hours(37)).await;
    task.abort();
    assert_eq!(
        *runs.lock().unwrap(),
        [
            at(5, 12, 0, 0),
            at(5, 12, 0, 30),
            at(5, 12, 1, 30),
            at(6, 0, 10, 0),
            at(7, 0, 10, 0),
        ]
    );
}
