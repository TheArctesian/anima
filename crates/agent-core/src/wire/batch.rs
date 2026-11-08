//! The push body: the points of one device, column by column (K8 Section 4).

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::points::{Point, SeriesKey};

/// `POST /agent/v1/push` (K8 Section 4). Parsed as given; [`Batch::check`] says whether the
/// hub takes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    /// The device from `Enrolled`; it must be the token's device (MET-017).
    pub device: Uuid,
    pub series: Vec<BatchSeries>,
    pub points: Vec<BatchPoint>,
    /// The device's GPUs, sent after start and after each discovery; absent means unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpus: Option<Vec<BatchGpu>>,
}

/// One column of a batch. `kind` is text, so a kind this hub does not know can be ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSeries {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<u8>,
}

/// One row of a batch: the window ending at `at`, one entry per series, `null` for a series
/// with no sample in the window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BatchPoint {
    #[serde(with = "super::times")]
    pub at: DateTime<Utc>,
    pub mean: Vec<Option<f64>>,
    /// Present when some column has a maximum; non-null only for CPU and GPU use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Vec<Option<f64>>>,
}

/// A GPU of the device (MET-006, MET-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchGpu {
    pub index: u8,
    pub name: String,
    pub vendor: GpuVendor,
    /// False for Intel and Apple, whose GPUs share system memory (MET-007).
    pub has_vram: bool,
}

/// Who made a GPU, as a batch spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
}

impl Batch {
    /// The batch of `points` for `device`: its series are every series of the points, in
    /// [`SeriesKey`] order, and a point holds `null` for a series it has no value of (K8
    /// Section 4). Give it a slice [`Backlog::batch`](crate::points::Backlog::batch) returned.
    #[must_use]
    pub fn new(device: Uuid, points: &[Point], gpus: Option<Vec<BatchGpu>>) -> Self {
        let keys: BTreeSet<&SeriesKey> = points
            .iter()
            .flat_map(|point| point.values().keys())
            .collect();
        Self {
            device,
            series: keys.iter().map(|key| BatchSeries::of(key)).collect(),
            points: points
                .iter()
                .map(|point| BatchPoint::of(point, &keys))
                .collect(),
            gpus,
        }
    }
}

impl BatchSeries {
    fn of(key: &SeriesKey) -> Self {
        Self {
            kind: key.kind().as_str().to_owned(),
            label: key.label().map(str::to_owned),
            gpu: key.gpu(),
        }
    }
}

impl BatchPoint {
    fn of(point: &Point, keys: &BTreeSet<&SeriesKey>) -> Self {
        let values = point.values();
        let mean = keys
            .iter()
            .map(|key| values.get(*key).map(|summary| summary.mean))
            .collect();
        let max: Vec<Option<f64>> = keys
            .iter()
            .map(|key| values.get(*key).and_then(|summary| summary.max))
            .collect();
        Self {
            at: point.at(),
            mean,
            max: max.iter().any(Option::is_some).then_some(max),
        }
    }
}
