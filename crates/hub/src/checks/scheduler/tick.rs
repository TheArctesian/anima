//! One pass of the scheduler: settle finished attempts, run the daily jobs when due, spread
//! first runs, report late checks and start due attempts within the limit (CHK-001,
//! CHK-010, HUB-003).
//!
//! One task owns this state, so no lock guards it. Attempts run in a `JoinSet`; dropping the
//! scheduler aborts them.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
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
    daily,
    schedule::{self, Watchdog},
};
use crate::{
    checks::{self, Due, store},
    devices,
    events::{self, NewEvent, Severity, Signal, Source},
    ids::{CheckId, DeviceId},
    status::Status,
};

/// The most due checks read in one tick.
const DUE_LIMIT: u32 = 256;
const DAY: TimeDelta = TimeDelta::hours(24);

/// The value, with a failure logged at this I/O boundary (LOG-007: errors hold no secret).
fn logged<T, E: Display>(what: &'static str, result: Result<T, E>) -> Option<T> {
    result
        .inspect_err(|error| tracing::warn!(domain = "checks", %error, "{what}"))
        .ok()
}

/// The value, with a failure about one device logged.
fn for_device<T, E: Display>(
    device: DeviceId,
    what: &'static str,
    result: Result<T, E>,
) -> Option<T> {
    result
        .inspect_err(|error| {
            tracing::warn!(domain = "checks", device_id = %device, %error, "{what}");
        })
        .ok()
}

pub(super) struct Scheduler {
    ctx: Arc<Context>,
    permits: Arc<Semaphore>,
    running: JoinSet<Finished>,
    /// The check of each running attempt; a check here is never started again.
    in_flight: HashMap<task::Id, CheckId>,
    watchdog: Watchdog,
    /// The last device status seen, seeded from the database before a device's first beat.
    known: HashMap<DeviceId, Status>,
    next_daily: Option<DateTime<Utc>>,
}

impl Scheduler {
    pub(super) fn new(ctx: Context, concurrency: usize) -> Self {
        Self {
            ctx: Arc::new(ctx),
            permits: Arc::new(Semaphore::new(concurrency)),
            running: JoinSet::new(),
            in_flight: HashMap::new(),
            watchdog: Watchdog::default(),
            known: HashMap::new(),
            next_daily: None,
        }
    }

    /// One pass at `now`. A failed read of the due checks skips the rest of the pass.
    pub(super) async fn tick(&mut self, now: DateTime<Utc>, rng: &mut impl Rng) {
        while let Some(joined) = self.running.try_join_next_with_id() {
            self.settle(joined, now).await;
        }
        if self.next_daily.is_none_or(|at| now >= at) {
            daily::run(&self.ctx.db, now).await;
            self.next_daily = Some(now + DAY);
        }
        let due = checks::due_checks(&self.ctx.db, now, DUE_LIMIT).await;
        let Some(due) = logged("the due checks cannot be read", due) else {
            return;
        };
        if !self.seed(&due, now).await {
            return;
        }
        for check in due {
            self.consider(check, now, rng).await;
        }
    }

    /// Reports the due check when it is late, then gives it its first due time or starts it,
    /// unless it is in flight or every permit is taken.
    async fn consider(&mut self, check: Due, now: DateTime<Utc>, rng: &mut impl Rng) {
        if self.watchdog.should_report(&check, now) {
            self.report_late(&check, now).await;
        }
        if self.in_flight.values().any(|id| *id == check.id) {
            return;
        }
        if check.state.next_due_at.is_none() {
            let at = schedule::first_due(&check, now, rng);
            if let Err(error) = store::set_next_due(&self.ctx.db, check.id, at).await {
                warn(&check, "the first due time cannot be stored", &error);
            }
            return;
        }
        if let Ok(permit) = Arc::clone(&self.permits).try_acquire_owned() {
            self.start(check, permit, now);
        }
    }

    /// Waits for every running attempt and settles it.
    #[cfg(test)]
    pub(super) async fn finish_running(&mut self, now: DateTime<Utc>) {
        while let Some(joined) = self.running.join_next_with_id().await {
            self.settle(joined, now).await;
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

    async fn settle(
        &mut self,
        joined: Result<(task::Id, Finished), task::JoinError>,
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
        if let Some(name) = finished.recorded {
            self.watchdog.clear(finished.check);
            self.publish_status(finished.device, &name, now).await;
        }
    }

    /// Reads the status of the due checks' devices not seen yet, before their next beat.
    /// Returns false when the read fails.
    async fn seed(&mut self, due: &[Due], now: DateTime<Utc>) -> bool {
        let unseen: Vec<DeviceId> = due
            .iter()
            .map(|check| check.device)
            .filter(|device| !self.known.contains_key(device))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if unseen.is_empty() {
            return true;
        }
        let statuses = checks::statuses(&self.ctx.db, &unseen, now).await;
        let Some(statuses) = logged("the device statuses cannot be read", statuses) else {
            return false;
        };
        self.known.extend(statuses);
        true
    }

    /// Publishes and records the device's status when it differs from the last one seen.
    async fn publish_status(&mut self, device: DeviceId, name: &str, now: DateTime<Utc>) {
        let status = checks::device_status(&self.ctx.db, device, now).await;
        let Some(status) = for_device(device, "the device status cannot be read", status) else {
            return;
        };
        if self.known.insert(device, status) == Some(status) {
            return;
        }
        let at = now;
        self.ctx
            .bus
            .publish(Signal::DeviceStatus { device, status, at });
        let event = NewEvent {
            severity: schedule::severity_of(status),
            source: Source::Check,
            device: Some((device, name.to_owned())),
            message: schedule::status_message(name, status),
        };
        let written = self.write_event(event, now).await;
        for_device(device, "the status change event cannot be written", written);
    }

    /// Writes the Error event for a late check (CHK-010). When it cannot be written, the
    /// next tick tries again.
    async fn report_late(&mut self, check: &Due, now: DateTime<Utc>) {
        let names = async {
            let name = checks::get(&self.ctx.db, check.id).await?.name;
            let device = devices::get(&self.ctx.db, check.device)
                .await
                .map_err(checks::from_devices)?
                .name;
            Ok::<_, checks::Error>((name, device))
        };
        let written = match names.await {
            Ok((name, device)) => {
                let event = NewEvent {
                    severity: Severity::Error,
                    source: Source::Check,
                    device: Some((check.device, device.clone())),
                    message: schedule::late_message(&name, &device),
                };
                self.write_event(event, now)
                    .await
                    .map_err(|e| e.to_string())
            }
            Err(error) => Err(error.to_string()),
        };
        if let Err(error) = written {
            warn(check, "the late check event cannot be written", &error);
            self.watchdog.clear(check.id);
        }
    }

    /// Writes one event in its own short transaction.
    async fn write_event(&self, event: NewEvent, now: DateTime<Utc>) -> Result<(), events::Error> {
        let mut tx = self.ctx.db.begin().await?;
        events::record(&mut tx, &self.ctx.bus, now, event).await?;
        tx.commit().await?;
        Ok(())
    }
}
