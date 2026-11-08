//! One pass of the scheduler: settle the devices whose checks had beats, spread first runs, report late checks and start due attempts within the limits
//! (CHK-001, CHK-010, HUB-003, LOG-004).
//!
//! One task owns this state, so no lock guards it. Attempts run in a `JoinSet`; dropping the
//! scheduler aborts them.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::{DateTime, TimeDelta, Utc};
use rand::Rng;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    task::{self, JoinSet},
};

use super::{
    attempt::{self, Context, Finished, warn},
    log::logged,
    notices,
    schedule::{self, Timing, Watchdog},
};
use crate::{
    checks::{self, Due, store},
    devices,
    ids::{CheckId, DeviceId},
    status::Status,
};

/// How far back each tick looks for new beats beyond the previous tick: a writer reads the
/// clock before it commits, so its beat can be older than a tick that missed it (LOG-004).
const BEAT_OVERLAP: TimeDelta = TimeDelta::seconds(5);
/// How long a device whose status could not be settled waits before the next try.
const SETTLE_RETRY: TimeDelta = TimeDelta::seconds(5);

/// How much one scheduler does at once.
#[derive(Clone, Copy, Debug)]
pub(super) struct Limits {
    /// Probes running at the same time.
    pub(super) probes: usize,
    pub(super) due_per_tick: u32,
}

pub(super) struct Scheduler {
    ctx: Arc<Context>,
    due_per_tick: u32,
    permits: Arc<Semaphore>,
    running: JoinSet<Finished>,
    /// The check of each running attempt; a check here is never started again.
    in_flight: HashMap<task::Id, CheckId>,
    watchdog: Watchdog,
    /// The last device status published, or read before the device's next beat.
    known: HashMap<DeviceId, Status>,
    /// Devices whose status must be compared with `known` again, and from when.
    unsettled: HashMap<DeviceId, DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    last_tick: Option<DateTime<Utc>>,
}

impl Scheduler {
    pub(super) fn new(ctx: Context, limits: Limits) -> Self {
        Self {
            ctx: Arc::new(ctx),
            due_per_tick: limits.due_per_tick,
            permits: Arc::new(Semaphore::new(limits.probes)),
            running: JoinSet::new(),
            in_flight: HashMap::new(),
            watchdog: Watchdog::default(),
            known: HashMap::new(),
            unsettled: HashMap::new(),
            started_at: None,
            last_tick: None,
        }
    }

    /// One pass at `now`. A failed read of the statuses or the due checks skips the rest.
    pub(super) async fn tick(&mut self, now: DateTime<Utc>, rng: &mut impl Rng) {
        let started = *self.started_at.get_or_insert(now);
        if self.last_tick.is_none() && !self.seed_every_device(now).await {
            return;
        }
        while let Some(joined) = self.running.try_join_next_with_id() {
            self.finish(joined, started, now);
        }
        self.collect_beaten(now).await;
        self.settle_devices(now).await;
        let due = checks::due_checks(&self.ctx.db, now, self.due_per_tick).await;
        let Some(due) = logged(None, "the due checks cannot be read", due) else {
            return;
        };
        if !self.seed(&due, now).await {
            return;
        }
        if due.len() < usize::try_from(self.due_per_tick).unwrap_or(usize::MAX) {
            self.watchdog
                .retain(&due.iter().map(|check| check.id).collect());
        }
        for check in due {
            self.consider(check, now, rng).await;
        }
    }

    /// Waits for every running attempt, then settles the devices.
    #[cfg(test)]
    pub(super) async fn finish_running(&mut self, now: DateTime<Utc>) {
        let started = *self.started_at.get_or_insert(now);
        while let Some(joined) = self.running.join_next_with_id().await {
            self.finish(joined, started, now);
        }
        self.settle_devices(now).await;
    }

    /// Reports the check when it is late, then spreads it or starts it, unless it is in
    /// flight or every permit is taken.
    async fn consider(&mut self, check: Due, now: DateTime<Utc>, rng: &mut impl Rng) {
        let started = self.started_at.unwrap_or(now);
        let late = schedule::is_late(Timing::from(&check), started, now);
        if self.watchdog.should_report(check.id, late)
            && !notices::report_late(&self.ctx, &check, now).await
        {
            self.watchdog.clear(check.id);
        }
        if self.in_flight.values().any(|id| *id == check.id) {
            return;
        }
        if schedule::needs_spreading(&check, started) {
            let at = schedule::first_due(&check, now, rng);
            let moved = store::set_next_due(&self.ctx.db, check.id, check.state.next_due_at, at);
            if let Err(error) = moved.await {
                warn(&check, "the first due time cannot be stored", &error);
            }
            return;
        }
        if let Ok(permit) = Arc::clone(&self.permits).try_acquire_owned() {
            self.start(check, permit, now);
        }
    }

    fn start(&mut self, check: Due, permit: OwnedSemaphorePermit, now: DateTime<Utc>) {
        let ctx = Arc::clone(&self.ctx);
        let id = check.id;
        let handle = self.running.spawn(async move {
            let _permit = permit;
            attempt::run(&ctx, check, now).await
        });
        self.in_flight.insert(handle.id(), id);
    }

    /// Books a finished attempt. A check that ran clears its lateness unless its new due
    /// time is late already (CHK-010).
    fn finish(
        &mut self,
        joined: Result<(task::Id, Finished), task::JoinError>,
        started: DateTime<Utc>,
        now: DateTime<Utc>,
    ) {
        let finished = match joined {
            Ok((task, finished)) => {
                self.in_flight.remove(&task);
                finished
            }
            Err(error) => {
                let check = self.in_flight.remove(&error.id());
                tracing::error!(domain = "checks", check_id = ?check, "a check attempt failed");
                return;
            }
        };
        if let Some(timing) = finished.recorded {
            self.unsettled.entry(finished.device).or_insert(now);
            if !schedule::is_late(timing, started, now) {
                self.watchdog.clear(finished.check);
            }
        }
    }

    /// Marks the devices whose checks had a beat since the last tick, less the overlap, from
    /// any writer such as the push route (LOG-004). `known` makes a second look free. On a
    /// failed read the same interval is read next time.
    async fn collect_beaten(&mut self, now: DateTime<Utc>) {
        if let Some(since) = self.last_tick {
            let beaten = store::devices_beaten_since(&self.ctx.db, since - BEAT_OVERLAP).await;
            let Some(beaten) = logged(None, "the devices with new beats cannot be read", beaten)
            else {
                return;
            };
            for device in beaten {
                self.unsettled.entry(device).or_insert(now);
            }
        }
        self.last_tick = Some(now);
    }

    /// Settles the devices whose turn has come; one that fails waits [`SETTLE_RETRY`].
    async fn settle_devices(&mut self, now: DateTime<Utc>) {
        let ready: Vec<DeviceId> = self
            .unsettled
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(device, _)| *device)
            .collect();
        for device in ready {
            if notices::settle(&self.ctx, &mut self.known, device, now).await {
                self.unsettled.remove(&device);
            } else {
                self.unsettled.insert(device, now + SETTLE_RETRY);
            }
        }
    }

    /// Whether the scheduler still tracks the device's status.
    #[cfg(test)]
    pub(super) fn tracks(&self, device: DeviceId) -> bool {
        self.known.contains_key(&device) || self.unsettled.contains_key(&device)
    }

    /// Reads the status of every device once, before the first tick's beats.
    async fn seed_every_device(&mut self, now: DateTime<Utc>) -> bool {
        let listed = devices::list(&self.ctx.db).await;
        let Some(listed) = logged(None, "the devices cannot be read", listed) else {
            return false;
        };
        let ids: Vec<DeviceId> = listed.iter().map(|device| device.id).collect();
        self.read_statuses(&ids, now).await
    }

    /// Reads the status of the due checks' devices not seen yet, before their next beat.
    async fn seed(&mut self, due: &[Due], now: DateTime<Utc>) -> bool {
        let unseen: HashSet<DeviceId> = due
            .iter()
            .map(|check| check.device)
            .filter(|device| !self.known.contains_key(device))
            .collect();
        if unseen.is_empty() {
            return true;
        }
        self.read_statuses(&unseen.into_iter().collect::<Vec<_>>(), now)
            .await
    }

    async fn read_statuses(&mut self, devices: &[DeviceId], now: DateTime<Utc>) -> bool {
        let statuses = checks::statuses(&self.ctx.db, devices, now).await;
        let Some(statuses) = logged(None, "the device statuses cannot be read", statuses) else {
            return false;
        };
        for (device, status) in statuses {
            self.known.entry(device).or_insert(status);
        }
        true
    }
}
