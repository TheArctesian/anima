//! The agent's buffer and its pushes: points wait here until the hub accepts them (MET-013,
//! MET-014, K8 Sections 5 to 7). The decisions are in [`pacing`](crate::push::pacing),
//! [`renewal`](crate::push::renewal) and [`answer`](crate::hub::answer); this module carries them out and
//! logs the changes of state (never a token or a body, LOG-007).

pub mod pacing;
pub mod renewal;

use std::time::Duration;

use agent_core::points::{BATCH_POINTS, Backlog, Point};
use agent_core::wire::{Batch, BatchGpu, RefusalCode};
use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;

use crate::hub::HubClient;
use crate::hub::answer::Answer;

use crate::report;
use crate::token_store::{Credential, TokenStore};
use pacing::{Action, Backoff, RHYTHM};

/// What the sampling side hands the pusher.
pub enum Event {
    Point(Point),
    /// The GPUs found at start or at a discovery; sent in the next batch (K8 Section 4).
    Gpus(Vec<BatchGpu>),
}

/// Everything the pusher keeps between pushes.
pub struct Pusher {
    hub: HubClient,
    device: Uuid,
    credential: Credential,
    store: TokenStore,
    backlog: Backlog,
    gpus: Option<Vec<BatchGpu>>,
    backoff: Backoff,
    /// The largest batch to try; smaller after `TOO_LARGE`.
    limit: usize,
    /// When a refused token is tried again; `Some` while pushing is stopped.
    stopped_until: Option<DateTime<Utc>>,
    renewal_tried: Option<DateTime<Utc>>,
}

impl Pusher {
    #[must_use]
    pub fn new(hub: HubClient, device: Uuid, credential: Credential, store: TokenStore) -> Self {
        Self {
            hub,
            device,
            credential,
            store,
            backlog: Backlog::new(),
            gpus: None,
            backoff: Backoff::default(),
            limit: BATCH_POINTS,
            stopped_until: None,
            renewal_tried: None,
        }
    }

    /// Takes what sampling produced.
    pub fn accept(&mut self, event: Event) {
        match event {
            Event::Point(point) => self.backlog.push(point),
            Event::Gpus(gpus) => self.gpus = Some(gpus),
        }
    }

    /// Whether pushing is stopped for a refused token (K8 Section 6).
    #[must_use]
    pub const fn is_stopped(&self) -> bool {
        self.stopped_until.is_some()
    }

    /// One round at `now`: renew when due, drop stale points, then send batches oldest first
    /// until none is left or one fails. Returns how long to wait before the next round.
    pub async fn push_due(&mut self, now: DateTime<Utc>) -> Duration {
        if self.stopped_until.is_none() {
            self.renew_if_due(now).await;
        }
        if let Some(retry_at) = self.stopped_until
            && !self.adopt_changed_token()
            && now < retry_at
        {
            return RHYTHM;
        }
        loop {
            self.backlog.drop_stale(now);
            let waiting = self.backlog.batch().points().len();
            if waiting == 0 {
                return RHYTHM;
            }
            match self.send(waiting.min(self.limit), now).await {
                Round::Sent => {}
                Round::Wait(wait) => return wait,
            }
        }
    }

    /// Sends one batch of at most `count` points and acts on the answer.
    async fn send(&mut self, count: usize, now: DateTime<Utc>) -> Round {
        let mut count = count;
        loop {
            let outgoing = self.backlog.batch().first(count);
            let Some(receipt) = outgoing.receipt() else {
                return Round::Sent;
            };
            let batch = Batch::new(self.device, outgoing.points(), self.gpus.clone());
            let answer = self.hub.push(&self.credential.token, &batch).await;
            match pacing::action(&answer) {
                Action::Remove => {
                    self.backlog.remove_through(receipt);
                    self.gpus = None;
                    self.limit = pacing::grown(count);
                    self.recovered();
                    return Round::Sent;
                }
                Action::Drop(code) => {
                    report::batch_dropped(code, count);
                    self.backlog.remove_through(receipt);
                    return Round::Sent;
                }
                Action::Shrink => {
                    let Some(smaller) = pacing::smaller(count) else {
                        report::batch_dropped(RefusalCode::TooLarge, 1);
                        self.backlog.remove_through(receipt);
                        return Round::Sent;
                    };
                    self.limit = smaller;
                    count = smaller;
                }
                Action::Stop(code) => return Round::Wait(self.stop(code, now)),
                Action::Wait => return Round::Wait(self.failed(&answer)),
            }
        }
    }

    fn failed(&mut self, answer: &Answer<()>) -> Duration {
        if !self.backoff.is_failing() {
            match answer {
                Answer::Failed(cause) => report::push_failed(*cause),
                _ => report::hub_cannot_store(),
            }
        }
        self.backoff.failed()
    }

    fn recovered(&mut self) {
        if self.backoff.is_failing() {
            report::hub_back();
        }
        if self.stopped_until.take().is_some() {
            report::token_accepted_again();
        }
        self.backoff.recovered();
    }

    /// Stops pushing for 10 minutes. Logs only when it was pushing before (K8 Section 6).
    fn stop(&mut self, code: RefusalCode, now: DateTime<Utc>) -> Duration {
        if self.stopped_until.is_none() {
            report::token_refused(code);
        }
        let wait = TimeDelta::from_std(pacing::REFUSED_RETRY).unwrap_or(TimeDelta::minutes(10));
        self.stopped_until = Some(now + wait);
        RHYTHM
    }

    /// When the token store holds another token than the one in use, takes it and resumes.
    fn adopt_changed_token(&mut self) -> bool {
        let Ok(Some(stored)) = self.store.load() else {
            return false;
        };
        if stored.token.hash() == self.credential.token.hash() {
            return false;
        }
        self.credential = stored;
        self.stopped_until = None;
        report::token_store_changed();
        true
    }

    /// Renews the token inside its last 30 days (K8 Section 5): the new token is stored first,
    /// and only then used. If it cannot be stored the current one stays.
    async fn renew_if_due(&mut self, now: DateTime<Utc>) {
        if !renewal::due(self.credential.expires_at, now, self.renewal_tried) {
            return;
        }
        self.renewal_tried = Some(now);
        match self.hub.renew(&self.credential.token).await {
            Answer::Accepted(renewed) => self.keep_renewed(Credential::from(renewed)),
            Answer::Refused(code) => report::renewal_refused(code),
            Answer::Failed(cause) => report::renewal_failed(cause),
        }
    }

    /// Stores the renewed token and only then uses it; on a failure the current one stays.
    fn keep_renewed(&mut self, next: Credential) {
        match self.store.save(&next) {
            Ok(()) => {
                self.credential = next;
                report::renewed();
            }
            Err(error) => report::renewal_not_stored(&error),
        }
    }
}

enum Round {
    /// A batch left the buffer; the next goes at once.
    Sent,
    Wait(Duration),
}
