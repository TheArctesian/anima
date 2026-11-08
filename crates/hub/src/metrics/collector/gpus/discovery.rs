//! When the hub looks for a device's GPUs, and how it numbers what it finds (MET-006, MET-007,
//! MET-009). Pure: the time comes in as a parameter.
//!
//! A look is per vendor. A vendor that answered replaces its own GPUs; a vendor whose read did
//! not finish keeps the GPUs known before and is looked at again the next round, so one hung
//! tool never hides the others' GPUs. A GPU keeps its device number while its vendor reports
//! it under the same number of its own (`nvidia-smi` index, DRM card number, `ioreg` order, 0
//! for Intel); a new GPU takes the lowest free number. Limitation: `nvidia-smi` renumbers the
//! GPUs after one drops off the bus, and the spec 6.6 query gives no PCI address to match on,
//! so a later NVIDIA GPU can then take over the series of the one that dropped.

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

/// How long after boot a look is provisional: a GPU driver may not be ready yet (a tool that
/// fails reads as no GPU), so the hub looks again each round until a look is this far from
/// boot.
pub(in crate::metrics::collector) const SETTLE: TimeDelta = TimeDelta::minutes(5);

/// A GPU the hub found, with its number in its vendor's own output, which matches each later
/// reading to it.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) struct Found {
    pub(in crate::metrics::collector) source: u32,
    pub(in crate::metrics::collector) gpu: Gpu,
}

/// What the hub knows of a device's GPUs: when it last looked at every vendor, the boot time
/// the device gave then, the GPUs by device number, and the vendors whose last read did not
/// finish.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) struct Discovered {
    pub(in crate::metrics::collector) found_at: DateTime<Utc>,
    pub(in crate::metrics::collector) boot_at: Option<DateTime<Utc>>,
    pub(in crate::metrics::collector) gpus: Vec<Found>,
    pub(in crate::metrics::collector) pending: Vec<GpuVendor>,
}

/// A look a collection makes: at every vendor of the device's system (`only` is `None`), or
/// only at the vendors still pending.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::metrics::collector) struct Look {
    pub(in crate::metrics::collector) only: Option<Vec<GpuVendor>>,
    pub(in crate::metrics::collector) at: DateTime<Utc>,
    pub(in crate::metrics::collector) boot_at: Option<DateTime<Utc>>,
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

/// The look of the collection at `now`, of a device whose boot time reads `boot_at`, if any.
/// Every vendor at the device's first collection since the hub started (nothing `known`), 24
/// hours after the last look or when the hub clock went back before it, after a restart (a
/// boot time later than the one seen then by more than [`RESTART_TOLERANCE`]), and while the
/// last look was within [`SETTLE`] of boot. Otherwise only the pending vendors, if any.
pub(in crate::metrics::collector) fn due(
    known: Option<&Discovered>,
    boot_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<Look> {
    let look = |only| Look {
        only,
        at: now,
        boot_at,
    };
    let Some(known) = known else {
        return Some(look(None));
    };
    let stale = now < known.found_at || now - known.found_at >= EVERY;
    let restarted = match (known.boot_at, boot_at) {
        (Some(was), Some(is)) => is - was > RESTART_TOLERANCE,
        _ => false,
    };
    let early = known
        .boot_at
        .is_some_and(|booted| known.found_at - booted < SETTLE);
    if stale || restarted || early {
        return Some(look(None));
    }
    (!known.pending.is_empty()).then(|| look(Some(known.pending.clone())))
}

impl Look {
    /// Whether this look reads `vendor`.
    pub(in crate::metrics::collector) fn includes(&self, vendor: GpuVendor) -> bool {
        self.only.as_ref().is_none_or(|only| only.contains(&vendor))
    }

    /// What the hub knows after this look, from what it `known` before and the `answers`.
    /// A vendor looked at that answered replaces its GPUs; one whose read did not finish
    /// keeps them and is pending. Known GPUs keep their number; new ones take the lowest free
    /// one, the first of a repeated own number, at most [`MAX_LABELS`] in all. Only NVIDIA and
    /// AMD GPUs that gave their memory have VRAM (MET-007).
    pub(in crate::metrics::collector) fn merge(
        &self,
        known: Option<&Discovered>,
        answers: &[(GpuVendor, Answer)],
    ) -> Discovered {
        let before: &[Found] = known.map_or(&[], |known| &known.gpus);
        let answered: Vec<(GpuVendor, &[GpuReading])> = answers
            .iter()
            .filter(|(vendor, _)| self.includes(*vendor))
            .filter_map(|(vendor, answer)| match answer {
                Answer::Gpus(gpus) => Some((*vendor, gpus.as_slice())),
                Answer::Unknown => None,
            })
            .collect();
        let replaced = |vendor: GpuVendor| answered.iter().any(|(each, _)| *each == vendor);
        let mut gpus: Vec<Found> = before
            .iter()
            .filter(|found| !replaced(found.gpu.vendor))
            .cloned()
            .collect();
        let mut new = Vec::new();
        for (vendor, readings) in &answered {
            new.extend(keep_vendor(&mut gpus, before, *vendor, readings));
        }
        for (vendor, reading) in new {
            if gpus.len() >= MAX_LABELS {
                break;
            }
            let free = (0..i32::MAX).find(|free| gpus.iter().all(|found| found.gpu.index != *free));
            gpus.extend(free.map(|index| found(vendor, reading, index)));
        }
        gpus.sort_by_key(|found| found.gpu.index);
        let pending = answers
            .iter()
            .filter(|(vendor, answer)| self.includes(*vendor) && matches!(answer, Answer::Unknown))
            .map(|(vendor, _)| *vendor)
            .collect();
        // A look at the pending vendors only does not restart the 24 hours.
        let (found_at, boot_at) = match (known, &self.only) {
            (Some(known), Some(_)) => (known.found_at, known.boot_at),
            _ => (self.at, self.boot_at),
        };
        Discovered {
            found_at,
            boot_at,
            gpus,
            pending,
        }
    }
}

/// Adds to `gpus` each of `vendor`'s `readings` known `before`, under the number it had, and
/// returns the others, new, the first of a repeated own number, in own-number order. Known
/// numbers are kept first, so a new GPU of an earlier vendor cannot take one.
fn keep_vendor<'a>(
    gpus: &mut Vec<Found>,
    before: &[Found],
    vendor: GpuVendor,
    readings: &'a [GpuReading],
) -> Vec<(GpuVendor, &'a GpuReading)> {
    let mut own: Vec<&GpuReading> = readings.iter().collect();
    own.sort_by_key(|reading| reading.index);
    own.dedup_by_key(|reading| reading.index);
    let mut new = Vec::new();
    for reading in own {
        let was = before
            .iter()
            .find(|found| found.gpu.vendor == vendor && found.source == reading.index);
        match was {
            Some(was) => gpus.push(found(vendor, reading, was.gpu.index)),
            None => new.push((vendor, reading)),
        }
    }
    new
}

fn found(vendor: GpuVendor, reading: &GpuReading, index: i32) -> Found {
    Found {
        source: reading.index,
        gpu: Gpu {
            index,
            name: reading.name.clone(),
            vendor,
            has_vram: reading.vram.is_some()
                && matches!(vendor, GpuVendor::Nvidia | GpuVendor::Amd),
        },
    }
}
