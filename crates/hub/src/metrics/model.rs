//! The K1 enums of metrics and the sample a collection produces.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

/// What a series measures (the API's `MetricKind`). Stored as [`MetricKind::as_str`].
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
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuPercent => "cpu_percent",
            Self::MemoryPercent => "memory_percent",
            Self::MemoryBytes => "memory_bytes",
            Self::DiskPercent => "disk_percent",
            Self::DiskBytes => "disk_bytes",
            Self::NetworkInBps => "network_in_bps",
            Self::NetworkOutBps => "network_out_bps",
            Self::UptimeSeconds => "uptime_seconds",
            Self::Load1 => "load_1",
            Self::Load5 => "load_5",
            Self::Load15 => "load_15",
            Self::GpuPercent => "gpu_percent",
            Self::VramPercent => "vram_percent",
            Self::VramBytes => "vram_bytes",
            Self::GpuTemperatureC => "gpu_temperature_c",
            Self::GpuPowerW => "gpu_power_w",
        }
    }

    /// The kind stored as `text`; `None` for text no kind has.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }

    /// Whether values of this kind are shares of a whole, kept within 0 to 100.
    #[must_use]
    pub const fn is_percent(self) -> bool {
        matches!(
            self,
            Self::CpuPercent
                | Self::MemoryPercent
                | Self::DiskPercent
                | Self::GpuPercent
                | Self::VramPercent
        )
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

/// One value of one series at one collection: the label is a mount point or an interface name,
/// and empty for the kinds that have none (D-5).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub kind: MetricKind,
    pub label: String,
    pub value: f64,
}

impl Sample {
    /// A sample of `value`, or `None` when it is not a finite number of at least zero.
    /// Percent kinds are held to at most 100.
    #[must_use]
    pub fn new(kind: MetricKind, label: &str, value: f64) -> Option<Self> {
        if !value.is_finite() || value.is_sign_negative() {
            return None;
        }
        let value = if kind.is_percent() {
            value.min(100.0)
        } else {
            value
        };
        Some(Self {
            kind,
            label: label.to_owned(),
            value,
        })
    }
}
