//! Readers of the control group files of a container (HMD-071): what the container of a hosted
//! hub may use and what it used, never a figure of the server that runs it (HMD-074).
//!
//! The files are those of cgroup v2 at the root of the container's own control group
//! (`/sys/fs/cgroup`): `memory.max`, `memory.current`, `memory.stat`, `cpu.max` and `cpu.stat`.

use super::{Cpu, Memory, lines};

/// Thousandths of a processor, the unit of [`allowed_processors`].
pub const MILLI: u64 = 1000;

/// The memory of the container against its own limit: the use is `memory.current` less the
/// file cache that the kernel can take back at once (`inactive_file` of `memory.stat`). `None`
/// when `memory.max` is `max` or does not read: a container with no limit has no total of its
/// own, and the total of the server must not stand in for it.
#[must_use]
pub fn memory(max: &[u8], current: &[u8], stat: &[u8]) -> Option<Memory> {
    let total_bytes = first_number(max).filter(|total| *total > 0)?;
    let current = first_number(current)?;
    let reclaimable = keyed(stat, "inactive_file").unwrap_or(0);
    Some(Memory {
        total_bytes,
        used_bytes: current.saturating_sub(reclaimable),
    })
}

/// The microseconds of processor time that the control group used (`usage_usec` of `cpu.stat`).
#[must_use]
pub fn cpu_usage_micros(stat: &[u8]) -> Option<u64> {
    keyed(stat, "usage_usec")
}

/// The thousandths of a processor that the container may use: the hard limit of `cpu.max`
/// (quota over period) when it has one and it is under `processors`, else `processors`, the
/// count the container may run on. A share of the time under load (`cpu.weight`) is no limit
/// and is not read.
#[must_use]
pub fn allowed_processors(cpu_max: &[u8], processors: u64) -> u64 {
    let all = processors.max(1).saturating_mul(MILLI);
    let quota = lines(cpu_max).next().and_then(|line| {
        let mut words = line.split_whitespace();
        let quota = words.next()?.parse::<u64>().ok()?;
        let period = words
            .next()?
            .parse::<u64>()
            .ok()
            .filter(|period| *period > 0)?;
        Some(quota.saturating_mul(MILLI) / period)
    });
    quota
        .filter(|quota| *quota > 0)
        .map_or(all, |quota| quota.min(all))
}

/// The processor of the container as counters that [`Cpu::Ticks`] takes: the time that the
/// allowed processors had since `elapsed_micros` began, and the part of it that the container
/// did not use. `used_micros` is the processor time used since the same moment.
#[must_use]
pub fn cpu_ticks(used_micros: u64, elapsed_micros: u64, allowed_milli: u64) -> Cpu {
    let total = elapsed_micros.saturating_mul(allowed_milli);
    let busy = used_micros.saturating_mul(MILLI);
    Cpu::Ticks {
        idle: total.saturating_sub(busy),
        total,
    }
}

/// The first number of the first complete line (`memory.max`, `memory.current`).
fn first_number(output: &[u8]) -> Option<u64> {
    lines(output)
        .next()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// The value of `key` in a file of `key value` lines (`memory.stat`, `cpu.stat`).
fn keyed(output: &[u8], key: &str) -> Option<u64> {
    lines(output).find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == key).then(|| words.next()?.parse().ok())?
    })
}
