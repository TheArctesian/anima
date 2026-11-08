//! When the hub looks for a device's GPUs, and how it numbers what it finds (MET-006, MET-007).
//! Pure: the time comes in as a parameter.

use std::time::Duration;

use agent_core::readings::GpuReading;
use chrono::{DateTime, TimeDelta, Utc};

use super::answers::Answer;
use crate::metrics::{Gpu, GpuVendor, MAX_LABELS, Reading};

/// How long a discovery holds before the hub looks again (MET-006).
pub(in crate::metrics::collector) const EVERY: TimeDelta = TimeDelta::hours(24);

/// How much later a device's boot time must read than at the last discovery to count as a
/// restart. The boot time is the collection's hub time minus the uptime the device gives, and
/// the uptime is read up to 20 seconds (the collection deadline) after that time, so one boot
/// reads up to 20 seconds apart, more when the hub clock is stepped. 60 seconds is three times
/// that. A false restart costs one extra look; a restart within 60 seconds of the last
/// discovery's boot waits for the 24-hour look.
pub(in crate::metrics::collector) const RESTART_TOLERANCE: TimeDelta = TimeDelta::seconds(60);

/// A GPU the hub found, with its number in its vendor's own output (`nvidia-smi` index, DRM
/// card number, `ioreg` order, 0 for Intel), which matches each later reading to it.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) struct Found {
    pub(in crate::metrics::collector) source: u32,
    pub(in crate::metrics::collector) gpu: Gpu,
}

/// The last finished look for a device's GPUs: its hub time, the boot time the device gave
/// then, and the GPUs, numbered for the device from 0.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) struct Discovered {
    pub(in crate::metrics::collector) found_at: DateTime<Utc>,
    pub(in crate::metrics::collector) boot_at: Option<DateTime<Utc>>,
    pub(in crate::metrics::collector) gpus: Vec<Found>,
}

/// The boot time of the device whose `reading` was taken at `at`: `at` less its uptime.
/// `None` without an uptime or when it does not fit the calendar.
pub(in crate::metrics::collector) fn boot_at(
    reading: &Reading,
    at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let uptime = Duration::try_from_secs_f64(reading.uptime_seconds?).ok()?;
    at.checked_sub_signed(TimeDelta::from_std(uptime).ok()?)
}

/// Whether the collection at `now` looks for GPUs: at the device's first collection since the
/// hub started (nothing `known`), 24 hours after the last look or when the hub clock went
/// back before it, and when the device's boot time `boot_at` is later than the one seen then
/// by more than [`RESTART_TOLERANCE`] (MET-006). A boot time missing either time is no restart.
pub(in crate::metrics::collector) fn due(
    known: Option<&Discovered>,
    boot_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    let Some(known) = known else {
        return true;
    };
    let stale = now < known.found_at || now - known.found_at >= EVERY;
    let restarted = match (known.boot_at, boot_at) {
        (Some(was), Some(is)) => is - was > RESTART_TOLERANCE,
        _ => false,
    };
    stale || restarted
}

/// The GPUs of `answers` taken at `at` from a device that booted at `boot_at`, numbered from 0
/// in the order of the answers and each vendor's own number, the first of a repeated number,
/// at most [`MAX_LABELS`]. Only NVIDIA and AMD GPUs that gave their memory have VRAM (MET-007).
/// `None` when an answer is unknown: the hub could not tell, so it keeps what it knew.
pub(in crate::metrics::collector) fn discovered(
    answers: &[(GpuVendor, Answer)],
    at: DateTime<Utc>,
    boot_at: Option<DateTime<Utc>>,
) -> Option<Discovered> {
    let mut readings: Vec<(GpuVendor, &GpuReading)> = Vec::new();
    for (vendor, answer) in answers {
        let Answer::Gpus(gpus) = answer else {
            return None;
        };
        let mut own: Vec<&GpuReading> = gpus.iter().collect();
        own.sort_by_key(|reading| reading.index);
        own.dedup_by_key(|reading| reading.index);
        readings.extend(own.into_iter().map(|reading| (*vendor, reading)));
    }
    readings.truncate(MAX_LABELS);
    let gpus = readings
        .into_iter()
        .zip(0..)
        .map(|((vendor, reading), index)| Found {
            source: reading.index,
            gpu: Gpu {
                index,
                name: reading.name.clone(),
                vendor,
                has_vram: reading.vram.is_some()
                    && matches!(vendor, GpuVendor::Nvidia | GpuVendor::Amd),
            },
        })
        .collect();
    Some(Discovered {
        found_at: at,
        boot_at,
        gpus,
    })
}
