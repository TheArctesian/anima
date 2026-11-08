//! Whether the hub takes a parsed batch: K8 Sections 4 and 6, in the order device, shape,
//! times (MET-017).
//!
//! Pure: the token's device and the hub's clock are parameters (CQ-021). The bearer, the size
//! and the JSON are the hub transport's to refuse first.

use std::collections::BTreeSet;

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;

use super::{Batch, BatchGpu, BatchPoint, BatchSeries, GpuVendor, RefusalCode};
use crate::points::{
    Kind, MAX_GPU_INDEX, MAX_POINTS, MAX_SERIES, Point, SeriesKey, Summary, is_label, is_window_end,
};

/// The oldest a point may be on the hub's clock (MET-017).
const OLDEST: TimeDelta = TimeDelta::hours(1);

/// The furthest ahead of the hub's clock a point may be (MET-017).
const AHEAD: TimeDelta = TimeDelta::seconds(60);

/// What the hub takes from a batch: the points of the series whose kind it knows, each with
/// its non-null values only, and the GPUs when the batch has them (K8 Section 4).
#[derive(Clone, Debug, PartialEq)]
pub struct Accepted {
    pub points: Vec<Point>,
    pub gpus: Option<Vec<BatchGpu>>,
}

impl Batch {
    /// What the hub takes from this batch, pushed with a token of `device` and read at `now`.
    ///
    /// # Errors
    /// The first refusal of K8 Section 6 it meets, in this order: `WrongDevice` when the batch
    /// names another device; `Malformed` when it breaks a rule of Section 4; `OutOfRange` when a
    /// point is more than 1 hour before `now` or more than 60 seconds after it (exactly 1 hour
    /// and exactly 60 seconds are taken). A series of a kind this hub does not know is held to
    /// the counts and to "no two equal" only; its column is ignored.
    pub fn check(self, device: Uuid, now: DateTime<Utc>) -> Result<Accepted, RefusalCode> {
        if self.device != device {
            return Err(RefusalCode::WrongDevice);
        }
        let keys = series_keys(&self.series).ok_or(RefusalCode::Malformed)?;
        let shaped = points_fit(&self.points, &keys) && gpus_fit(self.gpus.as_deref());
        if !shaped {
            return Err(RefusalCode::Malformed);
        }
        if !in_range(&self.points, now) {
            return Err(RefusalCode::OutOfRange);
        }
        let points = self
            .points
            .into_iter()
            .map(|point| accepted(point, &keys))
            .collect();
        Ok(Accepted {
            points,
            gpus: self.gpus,
        })
    }
}

/// The key of each column, `None` for a kind this hub does not know; `None` overall when the
/// series break Section 4.
fn series_keys(series: &[BatchSeries]) -> Option<Vec<Option<SeriesKey>>> {
    if !(1..=MAX_SERIES).contains(&series.len()) {
        return None;
    }
    let distinct: BTreeSet<(&str, Option<&str>, Option<u8>)> = series
        .iter()
        .map(|one| (one.kind.as_str(), one.label.as_deref(), one.gpu))
        .collect();
    if distinct.len() != series.len() {
        return None;
    }
    series
        .iter()
        .map(|one| match Kind::parse(&one.kind) {
            Some(kind) => SeriesKey::new(kind, one.label.clone(), one.gpu).map(Some),
            None => Some(None),
        })
        .collect()
}

/// 1 to 360 points, `at` strictly increasing and on multiples of 10 seconds, each with one
/// value per series that fits its kind.
fn points_fit(points: &[BatchPoint], keys: &[Option<SeriesKey>]) -> bool {
    let increasing = points.windows(2).all(|pair| pair[0].at < pair[1].at);
    (1..=MAX_POINTS).contains(&points.len())
        && increasing
        && points.iter().all(|point| point_fits(point, keys))
}

fn point_fits(point: &BatchPoint, keys: &[Option<SeriesKey>]) -> bool {
    let columns = keys.len();
    if !is_window_end(point.at) || point.mean.len() != columns {
        return false;
    }
    let max = match &point.max {
        Some(max) if max.len() == columns => max.as_slice(),
        Some(_) => return false,
        None => &[],
    };
    keys.iter().enumerate().all(|(column, key)| {
        let Some(key) = key else { return true };
        let max = max.get(column).copied().flatten();
        value_fits(key.kind(), point.mean[column], max)
    })
}

/// A mean, and a max only for the kinds that keep one, never without a mean or below it.
fn value_fits(kind: Kind, mean: Option<f64>, max: Option<f64>) -> bool {
    match (mean, max) {
        (_, Some(_)) if !kind.has_max() => false,
        (None, max) => max.is_none(),
        (Some(mean), None) => number_fits(kind, mean),
        (Some(mean), Some(max)) => number_fits(kind, mean) && number_fits(kind, max) && max >= mean,
    }
}

/// Finite, at least 0, and at most 100 for a percent kind. `-0` is refused, as a sample
/// refuses it; the agent never sends it.
fn number_fits(kind: Kind, value: f64) -> bool {
    value.is_finite() && !value.is_sign_negative() && (!kind.is_percent() || value <= 100.0)
}

/// Each GPU: index 0 to 63 and unique, a name of 1 to 128 bytes without a control character,
/// and no VRAM for Intel and Apple (MET-007).
fn gpus_fit(gpus: Option<&[BatchGpu]>) -> bool {
    let gpus = gpus.unwrap_or_default();
    let indexes: BTreeSet<u8> = gpus.iter().map(|gpu| gpu.index).collect();
    indexes.len() == gpus.len()
        && gpus.iter().all(|gpu| {
            let shares_memory = matches!(gpu.vendor, GpuVendor::Intel | GpuVendor::Apple);
            gpu.index <= MAX_GPU_INDEX && is_label(&gpu.name) && !(shares_memory && gpu.has_vram)
        })
}

/// The oldest point is at most 1 hour old and the newest at most 60 seconds ahead of `now`.
/// The points are in order, so the two ends decide.
fn in_range(points: &[BatchPoint], now: DateTime<Utc>) -> bool {
    let oldest_fits = points.first().is_some_and(|point| now - point.at <= OLDEST);
    let newest_fits = points.last().is_some_and(|point| point.at - now <= AHEAD);
    oldest_fits && newest_fits
}

/// The point the hub stores: the known columns with a mean.
fn accepted(point: BatchPoint, keys: &[Option<SeriesKey>]) -> Point {
    let max = point.max.unwrap_or_default();
    let values = keys
        .iter()
        .zip(point.mean)
        .enumerate()
        .filter_map(|(column, (key, mean))| {
            let summary = Summary {
                mean: mean?,
                max: max.get(column).copied().flatten(),
            };
            Some((key.clone()?, summary))
        })
        .collect();
    Point::new(point.at, values)
}
