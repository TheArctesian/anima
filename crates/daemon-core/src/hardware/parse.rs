//! The parse of what the hardware readers print or read (CAP-001, CAP-004, CAP-005).

use std::time::Duration;

use super::{ThermalState, UsableMemory, UsableSource};
use crate::readings::GpuVendor;

/// One mebibyte.
const MIB: u64 = 1024 * 1024;

/// Above this much memory, macOS lets the GPU wire three quarters of it by default; at or below
/// it, two thirds.
const LARGE_MEMORY: u64 = 36 * 1024 * MIB;

/// A person who used the keyboard or the pointer within this time is active (CAP-004).
pub const ACTIVE_WITHIN: Duration = Duration::from_mins(5);

/// The number on the first line of `sysctl -n <name>`.
#[must_use]
pub fn sysctl_number(output: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(output).ok()?;
    text.lines().next()?.trim().parse().ok()
}

/// The usable GPU memory of Apple silicon (CAP-005): the value of `iogpu.wired_limit_mb` when
/// the operator set one, else the default share of macOS: three quarters of a memory larger
/// than 36 GiB, two thirds of a smaller one. This default is the value Metal gives as
/// `recommendedMaxWorkingSetSize` on these machines; the daemon does not call Metal.
#[must_use]
pub fn usable_memory(memory_bytes: u64, wired_limit_mb: Option<u64>) -> UsableMemory {
    if let Some(limit) = wired_limit_mb.filter(|limit| *limit > 0) {
        return UsableMemory {
            bytes: limit.saturating_mul(MIB).min(memory_bytes),
            source: UsableSource::WiredLimit,
        };
    }
    let bytes = if memory_bytes > LARGE_MEMORY {
        memory_bytes / 4 * 3
    } else {
        memory_bytes / 3 * 2
    };
    UsableMemory {
        bytes,
        source: UsableSource::DefaultShare,
    }
}

/// The thermal state `pmset -g therm` reports: nominal when no thermal warning was recorded or
/// the CPU speed limit is 100; throttled when the limit is lower; `None` for output of another
/// shape.
#[must_use]
pub fn pmset_thermal(output: &[u8]) -> Option<ThermalState> {
    let text = std::str::from_utf8(output).ok()?;
    let limit = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "CPU_Speed_Limit").then(|| value.trim().parse::<u32>().ok())?
    });
    match limit {
        Some(limit) if limit < 100 => Some(ThermalState::Throttled),
        Some(_) => Some(ThermalState::Nominal),
        None if text.contains("No thermal warning level has been recorded") => {
            Some(ThermalState::Nominal)
        }
        None => None,
    }
}

/// Whether the operator is active, from `ioreg -c IOHIDSystem -d 4 -r -k HIDIdleTime`: the
/// first `"HIDIdleTime" = <nanoseconds>` is less than [`ACTIVE_WITHIN`].
#[must_use]
pub fn ioreg_active(output: &[u8]) -> Option<bool> {
    let text = std::str::from_utf8(output).ok()?;
    let idle: u64 = text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        key.trim_start_matches(|c: char| c == '|' || c.is_whitespace())
            .trim()
            .eq("\"HIDIdleTime\"")
            .then(|| value.trim().parse().ok())?
    })?;
    Some(Duration::from_nanos(idle) < ACTIVE_WITHIN)
}

/// The vendor of a PCI vendor ID as `/sys/class/drm/cardN/device/vendor` holds it (`0x10de`).
#[must_use]
pub fn pci_vendor(text: &str) -> Option<GpuVendor> {
    match text.trim() {
        "0x10de" => Some(GpuVendor::Nvidia),
        "0x1002" => Some(GpuVendor::Amd),
        "0x8086" => Some(GpuVendor::Intel),
        _ => None,
    }
}
