//! The K1 enums of metrics and their bridge to the stored kinds of [`agent_core::points`].
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use agent_core::points::{Kind, Sample};
use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};

/// What a series measures (the API's `MetricKind`). Stored as [`MetricKind::as_str`]: the name
/// of its [`Kind`], the one list of stored kind names. `gpu_memory_bytes` has no member yet
/// (K1 `@pending`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum MetricKind {
    CpuPercent,
    MemoryPercent,
    MemoryBytes,
    DiskPercent,
    DiskBytes,
    NetworkInBps,
    NetworkOutBps,
    UptimeSeconds,
    #[graphql(name = "LOAD_1")]
    Load1,
    #[graphql(name = "LOAD_5")]
    Load5,
    #[graphql(name = "LOAD_15")]
    Load15,
    GpuPercent,
    VramPercent,
    VramBytes,
    #[graphql(name = "GPU_TEMPERATURE_C")]
    GpuTemperatureC,
    #[graphql(name = "GPU_POWER_W")]
    GpuPowerW,
}

impl MetricKind {
    const ALL: [Self; 16] = [
        Self::CpuPercent,
        Self::MemoryPercent,
        Self::MemoryBytes,
        Self::DiskPercent,
        Self::DiskBytes,
        Self::NetworkInBps,
        Self::NetworkOutBps,
        Self::UptimeSeconds,
        Self::Load1,
        Self::Load5,
        Self::Load15,
        Self::GpuPercent,
        Self::VramPercent,
        Self::VramBytes,
        Self::GpuTemperatureC,
        Self::GpuPowerW,
    ];

    /// The text kept in the `series.kind` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        Kind::from(self).as_str()
    }

    /// The kind stored as `text`; `None` for text no kind has.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }

    /// The API's member for `kind`; `None` for a kind the API does not serve yet
    /// (`gpu_memory_bytes`, K8, CR-15).
    #[must_use]
    pub fn served(kind: Kind) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|metric| Kind::from(*metric) == kind)
    }
}

impl From<MetricKind> for Kind {
    fn from(kind: MetricKind) -> Self {
        match kind {
            MetricKind::CpuPercent => Self::CpuPercent,
            MetricKind::MemoryPercent => Self::MemoryPercent,
            MetricKind::MemoryBytes => Self::MemoryBytes,
            MetricKind::DiskPercent => Self::DiskPercent,
            MetricKind::DiskBytes => Self::DiskBytes,
            MetricKind::NetworkInBps => Self::NetworkInBps,
            MetricKind::NetworkOutBps => Self::NetworkOutBps,
            MetricKind::UptimeSeconds => Self::UptimeSeconds,
            MetricKind::Load1 => Self::Load1,
            MetricKind::Load5 => Self::Load5,
            MetricKind::Load15 => Self::Load15,
            MetricKind::GpuPercent => Self::GpuPercent,
            MetricKind::VramPercent => Self::VramPercent,
            MetricKind::VramBytes => Self::VramBytes,
            MetricKind::GpuTemperatureC => Self::GpuTemperatureC,
            MetricKind::GpuPowerW => Self::GpuPowerW,
        }
    }
}

/// The span a metric chart covers (MET-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum MetricRange {
    Hour,
    Day,
    Week,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
}

impl GpuVendor {
    /// The text kept in the `gpus.vendor` column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nvidia => "nvidia",
            Self::Amd => "amd",
            Self::Intel => "intel",
            Self::Apple => "apple",
        }
    }

    /// The vendor stored as `text`; `None` for text no vendor has.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Nvidia, Self::Amd, Self::Intel, Self::Apple]
            .into_iter()
            .find(|vendor| vendor.as_str() == text)
    }
}

/// A GPU found on a device (MET-006).
#[derive(Clone, Debug, PartialEq, Eq, SimpleObject)]
pub struct Gpu {
    pub index: i32,
    pub name: String,
    pub vendor: GpuVendor,
    /// False for integrated and Apple GPUs (MET-007).
    pub has_vram: bool,
}

/// One point of a chart: the mean of its bucket, with the lowest and highest value in it.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct MetricPoint {
    pub at: DateTime<Utc>,
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The points of one series over a range, oldest first (MET-003). Bytes are `f64`, as the API's
/// `Float` (they pass 2^31).
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct MetricSeries {
    pub kind: MetricKind,
    pub label: String,
    pub gpu_index: Option<i32>,
    pub points: Vec<MetricPoint>,
}

/// The latest values for a device card (MET-004, MET-008). Disk: the fullest mount.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct Readings {
    pub at: DateTime<Utc>,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub vram_percent: Option<f64>,
    pub vram_gpu: Option<Gpu>,
}

/// One value of a collection, as the API shows it.
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct MetricSample {
    pub kind: MetricKind,
    pub label: String,
    pub value: f64,
}

impl MetricSample {
    /// The API's form of `sample`; `None` while the API does not serve its kind
    /// ([`MetricKind::served`]).
    #[must_use]
    pub fn served(sample: &Sample) -> Option<Self> {
        Some(Self {
            kind: MetricKind::served(sample.kind)?,
            label: sample.label.clone(),
            value: sample.value,
        })
    }
}

/// What a collection added for a device: the live feed's item (MET-005).
#[derive(Clone, Debug, PartialEq, SimpleObject)]
pub struct MetricsAdded {
    pub device_id: ID,
    pub at: DateTime<Utc>,
    pub samples: Vec<MetricSample>,
    pub readings: Readings,
}
