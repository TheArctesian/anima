//! Finding the installation of the App the attempt made, and the task that watches the attempt
//! (K-E1 Section 10a).
//!
//! An installation ID from a browser redirect is not trusted: the hub asks GitHub about it with
//! the App's own token, and takes it only when GitHub found it, it belongs to this App and to
//! the App's owner, and it is not suspended. Anything else is ignored. While the App waits for
//! Install, the watch also asks for the App's installations every few seconds, so a closed tab
//! does not stop the flow. It backs off while GitHub does not answer, and a wake (the bound
//! browser back on `/installed`) never makes it ask more often than its [`Pace`] allows.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use ring::rand::SystemRandom;
use source_exchange::{Declaration, Installation};
use tokio::task::JoinHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{GITHUB, attempt::App, attempts::Attempts, finish::finish};
use crate::{
    clock::Clock,
    engine::sources::{
        Scheduling,
        acquisition::{
            app_key::AppSigner,
            installations::{Lookup, installation, installations},
        },
    },
};

/// Whether `found` is an installation of `app` in its owner's account that works.
#[must_use]
pub fn fits(found: &Installation, app: &App) -> bool {
    found.app_id == app.id && found.account_id == app.owner.id && !found.suspended
}

/// The App's own signer, from the key GitHub gave it; `None` for a key that cannot sign.
fn signer(app: &App, key: &Zeroizing<String>) -> Option<AppSigner> {
    AppSigner::read(app.id, key.as_bytes()).ok()
}

/// The installation `id` from a redirect, when GitHub says it [`fits`] `app`.
pub async fn verified(
    scheduling: &Scheduling,
    (app, key): (&App, &Zeroizing<String>),
    id: &str,
) -> Option<u64> {
    let declaration: Declaration = *scheduling.registry.declaration(GITHUB)?;
    let signer = signer(app, key)?;
    let now = scheduling.clock.now();
    let random = SystemRandom::new();
    let asked = installation(
        &scheduling.transport,
        &declaration,
        (&signer, id),
        (now, &random),
    )
    .await;
    match asked {
        Lookup::Found(found) if fits(&found, app) && found.id.to_string() == id => Some(found.id),
        _ => None,
    }
}

/// The first installation of `app` that [`fits`] it, from the App's list: `NotFound` when none
/// does, `Unusable` when the hub cannot ask.
async fn listed(scheduling: &Scheduling, (app, key): (&App, &Zeroizing<String>)) -> Lookup<u64> {
    let (Some(declaration), Some(signer)) = (
        scheduling.registry.declaration(GITHUB).copied(),
        signer(app, key),
    ) else {
        return Lookup::Unusable;
    };
    let now = scheduling.clock.now();
    let random = SystemRandom::new();
    match installations(&scheduling.transport, &declaration, &signer, (now, &random)).await {
        Lookup::Found(all) => (all.iter().find(|found| fits(found, app)))
            .map_or(Lookup::NotFound, |found| Lookup::Found(found.id)),
        Lookup::NotFound => Lookup::NotFound,
        Lookup::Unreachable => Lookup::Unreachable,
        Lookup::Unusable => Lookup::Unusable,
    }
}

/// The longest the watch waits between two asks while GitHub does not answer.
pub const BACKOFF_CAP: Duration = Duration::from_secs(300);

/// When the watch may ask GitHub. A scheduled ask comes one wait after the last: the poll
/// interval, doubled for each ask in a row that GitHub did not answer (a rate limit included),
/// at most [`BACKOFF_CAP`]; an answer sets it back. A wake asks sooner, but never sooner than
/// one wait after the last ask. Pure: the time is a parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    poll: Duration,
    last: Option<DateTime<Utc>>,
    misses: u32,
}

impl Pace {
    /// No ask yet, with the poll interval `poll`.
    #[must_use]
    pub const fn new(poll: Duration) -> Self {
        Self {
            poll,
            last: None,
            misses: 0,
        }
    }

    /// How long the next scheduled ask waits.
    #[must_use]
    pub fn wait(&self) -> Duration {
        let doubled = (1_u32.checked_shl(self.misses))
            .map_or(Duration::MAX, |factor| self.poll.saturating_mul(factor));
        doubled.min(BACKOFF_CAP.max(self.poll))
    }

    /// Whether a wake at `now` may ask GitHub.
    #[must_use]
    pub fn woken_may_ask(&self, now: DateTime<Utc>) -> bool {
        let wait = TimeDelta::from_std(self.wait()).unwrap_or(TimeDelta::MAX);
        self.last.is_none_or(|last| {
            last.checked_add_signed(wait)
                .is_some_and(|next| now >= next)
        })
    }

    /// The watch asked at `now`; `answered` unless GitHub was unreachable.
    pub const fn asked(&mut self, now: DateTime<Utc>, answered: bool) {
        self.last = Some(now);
        self.misses = if answered {
            0
        } else {
            self.misses.saturating_add(1)
        };
    }
}

/// Resolves at the next scheduled ask of `pace`, or at a wake it allows by `clock`.
async fn next_ask(attempts: &Attempts, pace: &Pace, clock: &dyn Clock) {
    let due = tokio::time::Instant::now() + pace.wait();
    loop {
        tokio::select! {
            () = tokio::time::sleep_until(due) => return,
            () = attempts.woken() => {
                if pace.woken_may_ask(clock.now()) {
                    return;
                }
            }
        }
    }
}

/// Starts the task that watches attempt `id`: at each ask of its [`Pace`], it expires the
/// attempt when its time is over and, while the App waits for Install, asks GitHub for its
/// installations. It ends when the attempt ends or is replaced; tasks end with the hub.
pub fn watch(scheduling: Scheduling, attempts: Attempts, id: Uuid) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut pace = Pace::new(attempts.poll());
        loop {
            next_ask(&attempts, &pace, scheduling.clock.as_ref()).await;
            let now = scheduling.clock.now();
            let seen = attempts.step(Some(id), now, |held| Ok((held.ended(), held.waiting_app())));
            let Ok((false, waiting)) = seen else {
                return;
            };
            let Some((app, key)) = waiting else {
                continue;
            };
            let listed = listed(&scheduling, (&app, &key)).await;
            pace.asked(now, listed != Lookup::Unreachable);
            if let Lookup::Found(found) = listed {
                finish(&scheduling, &attempts, (id, found)).await;
            }
        }
    })
}
