//! The last exposition built (D-24). One build serves every scrape for [`FRESH_FOR`], and
//! scrapes that arrive during a build wait for that build (single flight). The build runs in
//! its own task, so a scrape the client abandons neither cancels it nor starts another; it
//! fails after [`BUILD_LIMIT`]. A failed build answers every scrape for [`FAILED_FOR`] without
//! a rebuild. So the database sees at most one build at a time and one build per
//! [`FAILED_FOR`] while the builds fail.
//!
//! Time comes in as a parameter. No lock is held across an await: the mutex guards only the
//! swap of the current round, and waiters wait on the round's watch channel.

use std::{
    future::Future,
    sync::{Mutex, PoisonError},
    time::Duration,
};

use axum::body::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::watch;

/// How long a built exposition is served before a scrape builds the next (D-24).
pub(super) const FRESH_FOR: TimeDelta = TimeDelta::seconds(10);
/// How long a failed build answers scrapes before a scrape builds again (D-24).
pub(super) const FAILED_FOR: TimeDelta = TimeDelta::seconds(2);
/// How long a build may run before its round fails, so a hung statement ends.
pub(super) const BUILD_LIMIT: Duration = Duration::from_secs(30);

/// How a build ended.
#[derive(Clone, Debug)]
enum Outcome {
    Built(Bytes),
    Failed,
}

/// The end of a round's build: `None` while it runs.
type Ending = watch::Receiver<Option<Outcome>>;

/// One build and everyone waiting for it.
#[derive(Debug)]
struct Round {
    /// The instant the build was asked for.
    at: DateTime<Utc>,
    ending: Ending,
}

impl Round {
    /// Whether a scrape at `now` must start a new round: this one is finished and past the time
    /// its outcome is kept, or its task ended without an outcome (it panicked). A clock that
    /// moved back by the time kept or more also spends it.
    fn spent(&self, now: DateTime<Utc>) -> bool {
        let outcome = self.ending.borrow().clone();
        let kept_for = match outcome {
            None => return self.ending.has_changed().is_err(),
            Some(Outcome::Built(_)) => FRESH_FOR,
            Some(Outcome::Failed) => FAILED_FOR,
        };
        (now - self.at).abs() >= kept_for
    }
}

/// The current round, if any scrape has started one.
#[derive(Debug, Default)]
pub(super) struct Cache {
    current: Mutex<Option<Round>>,
}

impl Cache {
    /// The body of the current round, whose build `build` starts when the round is spent.
    /// `None` when that build failed. Dropping the returned future leaves the build running.
    pub(super) async fn body<F, Fut>(&self, now: DateTime<Utc>, build: F) -> Option<Bytes>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<String>> + Send + 'static,
    {
        let mut ending = self.join(now, build);
        let outcome = ending.wait_for(Option::is_some).await.ok()?.clone();
        match outcome {
            Some(Outcome::Built(body)) => Some(body),
            Some(Outcome::Failed) | None => None,
        }
    }

    /// The end of the round a scrape at `now` joins, after starting it if the current one is
    /// spent.
    fn join<F, Fut>(&self, now: DateTime<Utc>, build: F) -> Ending
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<String>> + Send + 'static,
    {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(round) = current.as_ref().filter(|round| !round.spent(now)) {
            return round.ending.clone();
        }
        let (sender, ending) = watch::channel(None);
        *current = Some(Round {
            at: now,
            ending: ending.clone(),
        });
        drop(current);
        tokio::spawn(run(build(), sender));
        ending
    }
}

/// Runs one build to its outcome, or to [`BUILD_LIMIT`], and tells the round.
async fn run(build: impl Future<Output = Option<String>>, sender: watch::Sender<Option<Outcome>>) {
    let outcome = match tokio::time::timeout(BUILD_LIMIT, build).await {
        Ok(Some(text)) => Outcome::Built(Bytes::from(text)),
        Ok(None) => Outcome::Failed,
        Err(elapsed) => {
            // A task's root is its boundary: no caller is left to hear of the timeout.
            tracing::error!(%elapsed, "the metrics exposition build took too long");
            Outcome::Failed
        }
    };
    sender.send_replace(Some(outcome));
}
