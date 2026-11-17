//! What one collection read from a device, before any rate is worked out.

use super::Drive;

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
    /// What a user may still write. On APFS every volume of a container reports the whole
    /// container as its total, so used plus available, not the total, is the volume's size.
    pub available_bytes: u64,
}

/// Byte counters of one network interface since it came up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub received_bytes: u64,
    pub sent_bytes: u64,
}

/// I/O counters of one block device since boot, as `/proc/diskstats` lists it (MET-028).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriveCounters {
    pub name: String,
    /// Sectors of 512 bytes, whatever the drive's own sector size.
    pub read_sectors: u64,
    pub written_sectors: u64,
    /// Milliseconds the device spent with I/O in flight (`io_ticks`).
    pub busy_ms: u64,
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
    /// The drives `lsblk` listed (MET-027); `None` when it gave nothing readable (macOS, a
    /// device without `lsblk`, a failed read), which is not the same as a device with none.
    pub drives: Option<Vec<Drive>>,
    pub drive_counters: Vec<DriveCounters>,
}
