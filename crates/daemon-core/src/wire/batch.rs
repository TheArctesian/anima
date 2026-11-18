//! The push body: the points of one device, column by column (K8 Section 4).

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{BatchAgentUsage, BatchInventory};
use crate::points::{Point, SeriesKey};
use crate::readings::{self, Drive};

/// `POST /daemon/v1/push` (K8 Section 4). Parsed as given; [`Batch::check`] says whether the
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
    /// The device's drives, sent after start and when they change; absent means unchanged
    /// (MET-027). A daemon older than drives never sends it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drives: Option<Vec<BatchDrive>>,
    /// The device's tool inventory, sent after start and when a probe result changes; absent
    /// means unchanged (CAP-027, CAP-029). A daemon older than the inventory never sends it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory: Option<BatchInventory>,
    /// The version of the `anima-daemon` that sent the batch (R-122, ROL-051). A daemon older
    /// than R-122 sends none, and a hub older than R-122 ignores it (K8 Section 1).
    #[serde(
        rename = "daemonVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub daemon_version: Option<String>,
    /// The use of the AI agent programs: the daemon's state and the session days whose counts
    /// changed; absent means nothing new (R-135, AIS-019). A daemon older than R-135 never
    /// sends it.
    #[serde(
        rename = "agentUsage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub agent_usage: Option<BatchAgentUsage>,
}

/// The lists a batch announces besides its points, each `None` when unchanged (K8 Section 4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Announcements {
    pub gpus: Option<Vec<BatchGpu>>,
    pub drives: Option<Vec<BatchDrive>>,
    pub inventory: Option<BatchInventory>,
    pub agent_usage: Option<BatchAgentUsage>,
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

/// A drive of the device and the operator's mounts on it (MET-027).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDrive {
    pub name: String,
    pub kind: DriveKind,
    pub size_bytes: u64,
    /// Empty when the drive is not mounted.
    pub mounts: Vec<String>,
}

/// What a drive is made of, as a batch spells it. A kind a newer daemon knows and this hub
/// does not reads as `disk`, so the batch is not refused for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveKind {
    Nvme,
    Ssd,
    Hdd,
    #[serde(other)]
    Disk,
}

impl From<&Drive> for BatchDrive {
    fn from(drive: &Drive) -> Self {
        Self {
            name: drive.name.clone(),
            kind: match drive.kind {
                readings::DriveKind::Nvme => DriveKind::Nvme,
                readings::DriveKind::Ssd => DriveKind::Ssd,
                readings::DriveKind::Hdd => DriveKind::Hdd,
                readings::DriveKind::Disk => DriveKind::Disk,
            },
            size_bytes: drive.size_bytes,
            mounts: drive.mounts.clone(),
        }
    }
}

impl From<BatchDrive> for Drive {
    fn from(drive: BatchDrive) -> Self {
        Self {
            name: drive.name,
            kind: match drive.kind {
                DriveKind::Nvme => readings::DriveKind::Nvme,
                DriveKind::Ssd => readings::DriveKind::Ssd,
                DriveKind::Hdd => readings::DriveKind::Hdd,
                DriveKind::Disk => readings::DriveKind::Disk,
            },
            size_bytes: drive.size_bytes,
            mounts: drive.mounts,
        }
    }
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
    /// Section 4). Give it the points of an [`Outgoing`](crate::points::Outgoing), and the
    /// GPUs, drives, inventory and agent usage to announce, if any.
    #[must_use]
    pub fn new<'a>(
        device: Uuid,
        points: impl IntoIterator<Item = &'a Point>,
        announced: Announcements,
    ) -> Self {
        let points: Vec<&Point> = points.into_iter().collect();
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
            gpus: announced.gpus,
            drives: announced.drives,
            inventory: announced.inventory,
            daemon_version: None,
            agent_usage: announced.agent_usage,
        }
    }

    /// This batch, signed with the version of the daemon that sends it (R-122).
    #[must_use]
    pub fn from_daemon(self, version: &str) -> Self {
        Self {
            daemon_version: Some(version.to_owned()),
            ..self
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
