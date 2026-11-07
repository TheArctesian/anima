//! One collection round: the devices due, at most [`AT_ONCE`] collections at a time, then each
//! device's reading kept for its next rates and a failure change reported (MET-001, D-4,
//! D-10, D-11).
//!
//! One task owns this state, so no lock guards it. Collections run as tasks (`fan_out`); dropping
//! the round, as aborting the collector does, aborts them and drops their links.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::task::JoinError;

use super::{
    AT_ONCE, Context,
    collection::{self, Failure, Finished},
    failures::{self, Change},
    fan_out::FanOut,
    outputs::Previous,
    plan,
};
use crate::{devices, events, ids::DeviceId};

/// What the collector remembers between rounds.
pub(super) struct Collector {
    ctx: Arc<Context>,
    /// Each due device's last reading, for its rates (D-4).
    previous: HashMap<DeviceId, Previous>,
    /// The due devices whose last collection failed (D-10).
    failing: HashSet<DeviceId>,
}

impl Collector {
    pub(super) fn new(ctx: Context) -> Self {
        Self {
            ctx: Arc::new(ctx),
            previous: HashMap::new(),
            failing: HashSet::new(),
        }
    }

    /// Collects every due device once. A device list that cannot be read skips the round.
    pub(super) async fn round(&mut self) {
        let listed = devices::list(&self.ctx.db).await;
        let Ok(listed) = listed.inspect_err(unlisted) else {
            return;
        };
        let due = plan::due(listed);
        self.keep_only(&due);
        let jobs = due
            .into_iter()
            .map(|device| {
                let previous = self.previous.get(&device.id).cloned();
                collection::collect(Arc::clone(&self.ctx), device, previous)
            })
            .collect();
        let mut collections = FanOut::new(jobs, AT_ONCE);
        while let Some(joined) = collections.next().await {
            self.finish(joined).await;
        }
    }

    /// Forgets the readings and failures of every device not in `due`: deleted, or no longer
    /// collected.
    fn keep_only(&mut self, due: &[devices::Device]) {
        let kept: HashSet<DeviceId> = due.iter().map(|device| device.id).collect();
        self.previous.retain(|device, _| kept.contains(device));
        self.failing.retain(|device| kept.contains(device));
    }

    /// Keeps a collection's reading, or reports its failure, and writes the event of a change.
    /// A change whose event cannot be written is left as it was, so the next round tries again.
    async fn finish(&mut self, joined: Result<Finished, JoinError>) {
        let Finished {
            device,
            name,
            result,
        } = match joined {
            Ok(finished) => finished,
            Err(error) => {
                // The panic message could hold device text, so only its kind is logged (LOG-007).
                let panicked = error.is_panic();
                tracing::warn!(domain = "metrics", panicked, "a collection stopped early");
                return;
            }
        };
        let was_failing = self.failing.contains(&device);
        let collected = match result {
            Ok(previous) => {
                self.previous.insert(device, previous);
                true
            }
            Err(Failure::Gone) => {
                self.failing.remove(&device);
                self.previous.remove(&device);
                return;
            }
            Err(error) => {
                failed(device, was_failing, &error);
                false
            }
        };
        if let Some(change) = failures::change(was_failing, collected)
            && !self.report(change, device, name).await
        {
            return;
        }
        if collected {
            self.failing.remove(&device);
        } else {
            self.failing.insert(device);
        }
    }

    /// Writes the event of `change` in its own short transaction; returns whether it was
    /// written.
    async fn report(&self, change: Change, device: DeviceId, name: String) -> bool {
        let now = self.ctx.clock.now();
        let event = failures::event(change, device, name);
        let written = async {
            let mut tx = self.ctx.db.begin().await?;
            events::record(&mut tx, &self.ctx.bus, now, event).await?;
            tx.commit().await?;
            Ok::<_, events::Error>(())
        };
        match written.await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(
                    domain = "metrics",
                    device_id = %device,
                    %error,
                    "the metrics failure event cannot be written"
                );
                false
            }
        }
    }
}

fn unlisted(error: &devices::Error) {
    tracing::warn!(domain = "metrics", %error, "the devices cannot be listed");
}

/// Logs a failed collection with the device id and the failure's fixed text only (LOG-007):
/// a warning when the device starts failing, a debug line on each repeat.
fn failed(device: DeviceId, was_failing: bool, error: &Failure) {
    if was_failing {
        failed_again(device, error);
    } else {
        tracing::warn!(domain = "metrics", device_id = %device, %error, "the metrics collection failed");
    }
}

fn failed_again(device: DeviceId, error: &Failure) {
    tracing::debug!(domain = "metrics", device_id = %device, %error, "the metrics collection failed again");
}
