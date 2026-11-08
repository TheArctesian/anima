//! What a series measures: the one list of stored kind names (MET-002, K8 Section 4).

/// What a series measures. Stored and sent as [`Kind::as_str`] (`series.kind`, K8 Section 4).
/// The hub's `MetricKind` is the API's view of the same list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    CpuPercent,
    MemoryPercent,
    MemoryBytes,
    DiskPercent,
    DiskBytes,
    NetworkInBps,
    NetworkOutBps,
    UptimeSeconds,
    Load1,
    Load5,
    Load15,
    GpuPercent,
    VramPercent,
    VramBytes,
    GpuTemperatureC,
    GpuPowerW,
    /// Memory in use by a unified-memory GPU (Apple). Never a VRAM value (MET-007, R-092 D-6).
    GpuMemoryBytes,
}

impl Kind {
    /// Every kind, in the order of the list above.
    pub const ALL: [Self; 17] = [
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
        Self::GpuMemoryBytes,
    ];

    /// The text kept in the `series.kind` column and sent in a batch's `kind`.
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
            Self::GpuMemoryBytes => "gpu_memory_bytes",
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

    /// Whether a series of this kind has a label: the mount point of a disk kind or the
    /// interface name of a network kind (K8 Section 4).
    #[must_use]
    pub const fn is_labelled(self) -> bool {
        matches!(
            self,
            Self::DiskPercent | Self::DiskBytes | Self::NetworkInBps | Self::NetworkOutBps
        )
    }

    /// Whether a series of this kind belongs to one GPU and carries its index (K8 Section 4).
    #[must_use]
    pub const fn is_per_gpu(self) -> bool {
        matches!(
            self,
            Self::GpuPercent
                | Self::VramPercent
                | Self::VramBytes
                | Self::GpuTemperatureC
                | Self::GpuPowerW
                | Self::GpuMemoryBytes
        )
    }

    /// Whether a point of this kind also holds the highest sample of its window: CPU and GPU
    /// use only (MET-014, K8 Section 4).
    #[must_use]
    pub const fn has_max(self) -> bool {
        matches!(self, Self::CpuPercent | Self::GpuPercent)
    }
}
