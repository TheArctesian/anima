//! One collection round: the operating system of each device at Other detected first, then the
//! devices due, at most [`AT_ONCE`] at a time, then each device's reading kept for its next
//! rates and a failure change reported (MET-001, DEV-025, D-4, D-10, D-11).
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
    collection::{self, Failure, Finished, Kept},
    detection,
    failures::{self, Change},
    fan_out::FanOut,
    plan,
};
use crate::{devices, events, ids::DeviceId};

/// What the collector remembers between rounds.
pub(super) struct Collector {
    ctx: Arc<Context>,
    /// Each due device's last reading, for its rates (D-4), and its known GPUs (MET-006).
    kept: HashMap<DeviceId, Kept>,
    /// The due devices whose last collection failed (D-10).
    failing: HashSet<DeviceId>,
}

impl Collector {
    pub(super) fn new(ctx: Context) -> Self {
        Self {
            ctx: Arc::new(ctx),
            kept: HashMap::new(),
            failing: HashSet::new(),
        }
    }

    /// Detects the operating system of every device awaiting it, then collects every due device
    /// once; a device detected now is collected from the next round (DEV-025). A device list
    /// that cannot be read skips the round.
    pub(super) async fn round(&mut self) {
        let listed = devices::list(&self.ctx.db).await;
        let Ok(listed) = listed.inspect_err(unlisted) else {
            return;
        };
        self.detect(plan::awaiting_os(&listed)).await;
        let due = plan::due(listed);
        self.keep_only(&due);
        let jobs = due
            .into_iter()
            .map(|device| {
                let kept = self.kept.get(&device.id).cloned();
                collection::collect(Arc::clone(&self.ctx), device, kept)
            })
            .collect();
        let mut collections = FanOut::new(jobs, AT_ONCE);
        while let Some(joined) = collections.next().await {
            self.finish(joined).await;
        }
    }

    /// Detects the operating system of each of `awaiting`, at most [`AT_ONCE`] at a time. A
    /// detection's outcome never reaches the failures or the events (DEV-025).
    async fn detect(&self, awaiting: Vec<devices::Device>) {
        let jobs = awaiting
            .into_iter()
            .map(|device| detection::detect(Arc::clone(&self.ctx), device))
            .collect();
        let mut detections = FanOut::new(jobs, AT_ONCE);
        while let Some(joined) = detections.next().await {
            if let Err(error) = joined {
                stopped(&error);
            }
        }
    }

    /// Forgets the readings, GPUs and failures of every device not in `due`: deleted, or no
    /// longer collected. Collected again, it starts with a new discovery that numbers its GPUs
    /// afresh.
    fn keep_only(&mut self, due: &[devices::Device]) {
        let kept: HashSet<DeviceId> = due.iter().map(|device| device.id).collect();
        self.kept.retain(|device, _| kept.contains(device));
        self.failing.retain(|device| kept.contains(device));
    }

    /// Makes the next collection of `device` look for its GPUs again, keeping their numbers: a
    /// failed collection may have stored a discovery without its samples, or missed a restart
    /// (MET-006, MET-009).
    fn look_again(&mut self, device: DeviceId) {
        if let Some(gpus) = self
            .kept
            .get_mut(&device)
            .and_then(|kept| kept.gpus.as_mut())
        {
            gpus.stale = true;
        }
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
                stopped(&error);
                return;
            }
        };
        let was_failing = self.failing.contains(&device);
        let collected = match result {
            Ok(kept) => {
                self.kept.insert(device, kept);
                true
            }
            Err(Failure::Gone) => {
                self.failing.remove(&device);
                self.kept.remove(&device);
                return;
            }
            Err(error) => {
                failed(device, was_failing, &error);
                self.look_again(device);
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

/// Logs a collection or detection that stopped early. The panic message could hold device
/// text, so only its kind is logged (LOG-007).
fn stopped(error: &JoinError) {
    let panicked = error.is_panic();
    tracing::warn!(domain = "metrics", panicked, "a collection stopped early");
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
