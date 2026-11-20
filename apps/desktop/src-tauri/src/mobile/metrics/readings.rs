use std::{fs, path::Path};

use chrono::{DateTime, Utc};
use daemon_core::{
    points::Kind,
    wire::{Batch, BatchPoint, BatchSeries},
};
use uuid::Uuid;

fn memory() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let kib = |key| {
        let line = text.lines().find_map(|line| line.strip_prefix(key))?;
        line.split_whitespace().next()?.parse::<u64>().ok()
    };
    let total = kib("MemTotal:")?.checked_mul(1024)?;
    let available = kib("MemAvailable:")?.checked_mul(1024)?;
    Some((total, total.checked_sub(available)?))
}

fn storage(path: &Path) -> Option<(u64, u64)> {
    let stats = rustix::fs::statvfs(path).ok()?;
    let total = stats.f_blocks.checked_mul(stats.f_frsize)?;
    let used = stats
        .f_blocks
        .checked_sub(stats.f_bfree)?
        .checked_mul(stats.f_frsize)?;
    Some((total, used))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the metrics wire carries byte counts as floats"
)]
fn add(
    series: &mut Vec<BatchSeries>,
    values: &mut Vec<Option<f64>>,
    kinds: (Kind, Kind),
    label: Option<&str>,
    (total, used): (u64, u64),
) {
    if total == 0 {
        return;
    }
    for (kind, value) in [
        (kinds.0, used as f64),
        (kinds.1, 100.0 * used as f64 / total as f64),
    ] {
        series.push(BatchSeries {
            kind: kind.as_str().to_owned(),
            label: label.map(str::to_owned),
            gpu: None,
        });
        values.push(Some(value));
    }
}

pub(super) fn batch(device: Uuid, now: DateTime<Utc>, path: &Path) -> Option<Batch> {
    let mut series = Vec::new();
    let mut mean = Vec::new();
    if let Some(reading) = storage(path) {
        add(
            &mut series,
            &mut mean,
            (Kind::DiskBytes, Kind::DiskPercent),
            Some("/data"),
            reading,
        );
    }
    if let Some(reading) = memory() {
        add(
            &mut series,
            &mut mean,
            (Kind::MemoryBytes, Kind::MemoryPercent),
            None,
            reading,
        );
    }
    if series.is_empty() {
        return None;
    }
    let seconds = now.timestamp();
    let at = DateTime::from_timestamp(seconds - seconds.rem_euclid(10), 0)?;
    Some(Batch {
        device,
        series,
        points: vec![BatchPoint {
            at,
            mean,
            max: None,
        }],
        gpus: None,
        drives: None,
        inventory: None,
        daemon_version: None,
        agent_usage: None,
        keys: None,
        update: None,
    })
}
