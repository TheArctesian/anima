//! Readers of the Linux `/proc` files (MET-002).

use super::{Cpu, DriveCounters, Interface, Load, Memory, label, lines, load, number};

/// The aggregate `cpu` line of `/proc/stat`: `total` is user, nice, system, idle, iowait, irq,
/// softirq and steal (guest time is already in user); `idle` is idle plus iowait. Fields an
/// old kernel lacks count as zero; fewer than four fields, or a bad one, gives `None`.
#[must_use]
pub fn cpu(output: &[u8]) -> Option<Cpu> {
    lines(output).find_map(|line| {
        let mut words = line.split_whitespace();
        if words.next()? != "cpu" {
            return None;
        }
        let counters = words
            .take(8)
            .map(|word| word.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;
        let idle = counters
            .get(3)?
            .checked_add(counters.get(4).copied().unwrap_or(0))?;
        let total = counters
            .iter()
            .try_fold(0_u64, |sum, count| sum.checked_add(*count))?;
        Some(Cpu::Ticks { idle, total })
    })
}

/// `/proc/meminfo`: used is `MemTotal` less `MemAvailable`, or less `MemFree + Buffers +
/// Cached` when the kernel has no `MemAvailable`. `None` without a non-zero total.
#[must_use]
pub fn memory(output: &[u8]) -> Option<Memory> {
    let (mut total, mut available) = (None, None);
    let (mut free, mut buffers, mut cached) = (None, None, None);
    for line in lines(output) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let slot = match key {
            "MemTotal" => &mut total,
            "MemAvailable" => &mut available,
            "MemFree" => &mut free,
            "Buffers" => &mut buffers,
            "Cached" => &mut cached,
            _ => continue,
        };
        *slot = slot.or_else(|| kibibytes(value));
    }
    let total_bytes = total.filter(|total| *total > 0)?;
    let available = available.or_else(|| {
        let reclaimable = free?.checked_add(buffers?)?;
        reclaimable.checked_add(cached?)
    })?;
    Some(Memory {
        total_bytes,
        used_bytes: total_bytes.saturating_sub(available),
    })
}

/// The bytes of a `/proc/meminfo` value such as `  16384000 kB`.
fn kibibytes(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}

/// `/proc/net/dev`: received and sent bytes per interface. A line that does not read is
/// skipped, as is an interface with an unusable name.
#[must_use]
pub fn interfaces(output: &[u8]) -> Vec<Interface> {
    lines(output)
        .filter_map(|line| {
            let (name, counters) = line.split_once(':')?;
            let mut columns = counters.split_whitespace();
            let received_bytes = columns.next()?.parse().ok()?;
            // Seven more receive columns, then the sent bytes.
            let sent_bytes = columns.nth(7)?.parse().ok()?;
            Some(Interface {
                name: label(name.trim())?.to_owned(),
                received_bytes,
                sent_bytes,
            })
        })
        .collect()
}

/// `/proc/uptime`: seconds since boot, the first number.
#[must_use]
pub fn uptime_seconds(output: &[u8]) -> Option<f64> {
    number(lines(output).next()?.split_whitespace().next()?)
}

/// `/proc/loadavg`: the 1, 5 and 15 minute averages, the first three numbers.
#[must_use]
pub fn load_average(output: &[u8]) -> Option<Load> {
    load(lines(output).next()?.split_whitespace())
}

/// `/proc/diskstats`: the read and written sectors and the busy milliseconds of each block
/// device (MET-028). A line with fewer than the 13 columns of kernel 2.6, a counter that does
/// not read or an unusable name is skipped.
#[must_use]
pub fn drive_counters(output: &[u8]) -> Vec<DriveCounters> {
    lines(output)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().take(13).collect();
            let counter = |at: usize| columns.get(at)?.parse::<u64>().ok();
            Some(DriveCounters {
                name: label(columns.get(2)?)?.to_owned(),
                read_sectors: counter(5)?,
                written_sectors: counter(9)?,
                busy_ms: counter(12)?,
            })
        })
        .collect()
}
