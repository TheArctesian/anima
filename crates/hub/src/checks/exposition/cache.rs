//! The last exposition built, served for [`FRESH_FOR`], so a token holder who scrapes in a
//! loop cannot load the database (D-24). Scrapes that arrive during a build wait for that one
//! build (single flight); a failed build fails its waiters too and is not kept.
//!
//! Time comes in as a parameter. No lock is held across an await: the mutex guards only the
//! swap of the current round, and the waiting happens on the round's `OnceCell`.

use std::{
    future::Future,
    sync::{Arc, Mutex, PoisonError},
};

use axum::body::Bytes;
use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::OnceCell;

/// How long a built exposition is served before a scrape builds the next (D-24).
pub(super) const FRESH_FOR: TimeDelta = TimeDelta::seconds(10);

/// A finished build: the text and the instant it was asked for.
#[derive(Debug)]
struct Built {
    at: DateTime<Utc>,
    body: Bytes,
}

/// One build and everyone waiting for it. `None` inside the cell is a failed build.
type Round = OnceCell<Option<Built>>;

/// The current round. A round that holds a stale body, or a failure, is replaced by the next
/// scrape; a round still building is joined.
#[derive(Debug, Default)]
pub(super) struct Cache {
    current: Mutex<Arc<Round>>,
}

impl Cache {
    /// The body built at most [`FRESH_FOR`] before `now`, or the body of the build in progress,
    /// or a new body from `build`. `None` when the build this scrape waited for failed.
    pub(super) async fn body<F, Fut>(&self, now: DateTime<Utc>, build: F) -> Option<Bytes>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<String>>,
    {
        let round = self.round(now);
        let finished = round
            .get_or_init(|| async move {
                build().await.map(|text| Built {
                    at: now,
                    body: Bytes::from(text),
                })
            })
            .await;
        finished.as_ref().map(|done| done.body.clone())
    }

    /// The round a scrape at `now` joins: the current one unless it is finished and either
    /// failed or stale. A clock that moved back by [`FRESH_FOR`] or more also makes it stale.
    fn round(&self, now: DateTime<Utc>) -> Arc<Round> {
        let mut current = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        let spent = current.get().is_some_and(|built| {
            built
                .as_ref()
                .is_none_or(|built| (now - built.at).abs() >= FRESH_FOR)
        });
        if spent {
            *current = Arc::new(Round::new());
        }
        Arc::clone(&current)
    }
}
