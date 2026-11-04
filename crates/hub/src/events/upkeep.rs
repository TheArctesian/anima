//! When a daily upkeep job runs: at start, then each day at 00:10 UTC, and sooner after a
//! failure, with a doubling delay (D-4, DB-004, DB-005).

use std::{future::Future, sync::Arc, time::Duration};

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};

use crate::clock::Clock;

/// The UTC time of day of the daily run. Partitions exist days ahead, so the time is not tied
/// to the day rollover.
const DAILY_AT: NaiveTime = NaiveTime::from_hms_opt(0, 10, 0).expect("a valid time of day");
/// The wait after the first failure; each further failure doubles it.
const FIRST_RETRY: Duration = Duration::from_secs(30);
/// The longest wait between two retries.
const LONGEST_RETRY: Duration = Duration::from_mins(30);
/// Failures in a row after which a job waits for its next daily run instead.
const RETRIES: u32 = 8;

/// The first daily run strictly after `now`.
pub(super) fn next_daily_run(now: DateTime<Utc>) -> DateTime<Utc> {
    let today = now.date_naive().and_time(DAILY_AT).and_utc();
    if today > now {
        today
    } else {
        today + TimeDelta::days(1)
    }
}

/// The wait after the `failures`-th failure in a row; `None` without a failure and after
/// [`RETRIES`] of them.
pub(super) fn retry_delay(failures: u32) -> Option<Duration> {
    if failures == 0 || failures > RETRIES {
        return None;
    }
    let doubled = FIRST_RETRY.saturating_mul(1 << (failures - 1));
    Some(doubled.min(LONGEST_RETRY))
}

/// After a run that leaves `failures` failures in a row at `now`: how long to wait, and the
/// count the next run starts from. A retry never waits past the next daily run; a job that has
/// used its retries waits for the daily run and counts from zero again.
pub(super) fn plan(now: DateTime<Utc>, failures: u32) -> (Duration, u32) {
    let until_daily = (next_daily_run(now) - now)
        .to_std()
        .unwrap_or(Duration::ZERO);
    match retry_delay(failures) {
        Some(retry) => (retry.min(until_daily), failures),
        None => (until_daily, 0),
    }
}

/// Runs `job` now and then on the [`plan`], with the time from `clock`, until the task is
/// aborted. A failure is written to the process log as `name` and the job's fault word only,
/// never its error text (LOG-007, API-010). Dropping the future between runs or inside one is
/// safe: an unfinished transaction rolls back.
pub async fn run_daily<J, F>(name: &'static str, clock: Arc<dyn Clock>, mut job: J)
where
    J: FnMut(DateTime<Utc>) -> F,
    F: Future<Output = Result<(), &'static str>>,
{
    let mut failures = 0;
    loop {
        failures = match job(clock.now()).await {
            Ok(()) => 0,
            Err(fault) => {
                let failures = failures + 1;
                tracing::warn!(job = name, fault, failures, "daily job failed");
                failures
            }
        };
        let (wait, carried) = plan(clock.now(), failures);
        failures = carried;
        tokio::time::sleep(wait).await;
    }
}

#[cfg(test)]
mod tests;
