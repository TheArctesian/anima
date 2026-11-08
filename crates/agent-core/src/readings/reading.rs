//! What one collection read from a device, before any rate is worked out.

/// CPU as the device reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cpu {
    /// Linux: counters since boot (`/proc/stat`). `idle` includes waiting for I/O.
    Ticks { idle: u64, total: u64 },
    /// macOS: the busy share over the last second, 0 to 100 (`top`).
    Percent(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memory {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Load {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

/// One mounted file system, as `df` lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disk {
    pub filesystem: String,
    pub mount: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

/// Byte counters of one network interface since it came up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub received_bytes: u64,
    pub sent_bytes: u64,
}

/// Everything read in one collection. A field is empty when its command gave nothing usable.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    pub cpu: Option<Cpu>,
    pub memory: Option<Memory>,
    pub load: Option<Load>,
    pub uptime_seconds: Option<f64>,
    pub disks: Vec<Disk>,
    pub interfaces: Vec<Interface>,
}
