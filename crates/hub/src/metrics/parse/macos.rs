//! Readers of macOS command output (MET-002).

use chrono::{DateTime, Utc};

use super::{label, lines, load, number, positional_lines};
use crate::metrics::reading::{Cpu, Interface, Load};

/// What `sysctl -n hw.memsize hw.pagesize vm.loadavg kern.boottime` printed, one value per line
/// in that order. A line that does not read leaves its field empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sysctl {
    pub memory_bytes: Option<u64>,
    pub page_size: Option<u64>,
    pub load: Option<Load>,
    /// Seconds since the Unix epoch at boot.
    pub boot_unix: Option<i64>,
}

impl Sysctl {
    /// Seconds from boot to `now`; `None` without a boot time or when it is after `now`.
    #[must_use]
    pub fn uptime_seconds(&self, now: DateTime<Utc>) -> Option<f64> {
        let seconds = now.timestamp().checked_sub(self.boot_unix?)?;
        let seconds = u32::try_from(seconds).ok()?;
        Some(f64::from(seconds))
    }
}

#[must_use]
pub fn sysctl(output: &[u8]) -> Sysctl {
    let mut rows = positional_lines(output);
    let mut next = || rows.next().flatten().map(str::trim);
    let memory_bytes = next().and_then(|text| text.parse().ok());
    let page_size = next().and_then(|text| text.parse().ok());
    let load = next()
        .and_then(braced)
        .and_then(|text| load(text.split_whitespace()));
    let boot_unix = next().and_then(boot_seconds);
    Sysctl {
        memory_bytes,
        page_size,
        load,
        boot_unix,
    }
}

/// The inside of `{ ... }`.
fn braced(text: &str) -> Option<&str> {
    text.strip_prefix('{')?.split('}').next()
}

/// `sec` of `{ sec = 1730800000, usec = 123456 } Tue Nov  5 ...`.
fn boot_seconds(text: &str) -> Option<i64> {
    let (_, after) = braced(text)?.split_once("sec =")?;
    after.split(',').next()?.trim().parse().ok()
}

/// Memory in use, in bytes, from `vm_stat`, as Activity Monitor counts it: app memory
/// (anonymous pages less purgeable pages) plus wired plus compressed pages, times the page
/// size. Without an "Anonymous pages" line, active pages stand for app memory. The page size is
/// the one in the header; `fallback_page_size` (sysctl `hw.pagesize`) is used when the header
/// has none. `None` without wired and compressed counts.
#[must_use]
pub fn memory_used_bytes(output: &[u8], fallback_page_size: Option<u64>) -> Option<u64> {
    let mut page_size = None;
    let [mut anonymous, mut purgeable, mut active] = [None; 3];
    let [mut wired, mut compressed] = [None; 2];
    for line in lines(output) {
        if let Some((_, after)) = line.split_once("page size of ") {
            page_size = page_size.or_else(|| after.split_whitespace().next()?.parse().ok());
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let slot = match key.trim() {
            "Anonymous pages" => &mut anonymous,
            "Pages purgeable" => &mut purgeable,
            "Pages active" => &mut active,
            "Pages wired down" => &mut wired,
            "Pages occupied by compressor" => &mut compressed,
            _ => continue,
        };
        *slot = slot.or_else(|| value.trim().trim_end_matches('.').parse::<u64>().ok());
    }
    let app = match anonymous {
        Some(anonymous) => anonymous.saturating_sub(purgeable.unwrap_or(0)),
        None => active?,
    };
    let pages = app.checked_add(wired?)?.checked_add(compressed?)?;
    pages.checked_mul(page_size.or(fallback_page_size)?)
}

/// `netstat -ib -n`: received and sent bytes per interface, from the `<Link#n>` row only (the
/// other rows repeat the counters once per address). The counters are the fifth and second
/// columns from the right, since a link row may have no address. A trailing `*` (interface
/// down) is dropped from the name.
#[must_use]
pub fn interfaces(output: &[u8]) -> Vec<Interface> {
    lines(output)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if !columns.get(2)?.starts_with("<Link#") {
                return None;
            }
            let count = columns.len();
            let received_bytes = columns.get(count.checked_sub(5)?)?.parse().ok()?;
            let sent_bytes = columns.get(count.checked_sub(2)?)?.parse().ok()?;
            Some(Interface {
                name: label(columns.first()?.trim_end_matches('*'))?.to_owned(),
                received_bytes,
                sent_bytes,
            })
        })
        .collect()
}

/// `top -l 2 -n 0 -s 1`: busy share is 100 less the idle share of the last `CPU usage:` line. The
/// first such line is the average since boot, so one line alone (output cut early) gives `None`,
/// as does a last line that does not read.
#[must_use]
pub fn cpu(output: &[u8]) -> Option<Cpu> {
    let mut usage = lines(output).filter(|line| line.trim_start().starts_with("CPU usage:"));
    let _since_boot = usage.next()?;
    let line = usage.last()?;
    let (before, _) = line.split_once("% idle")?;
    let idle = number(
        before
            .rsplit(|c: char| c.is_whitespace() || c == ',')
            .next()?,
    )?;
    (idle <= 100.0).then_some(Cpu::Percent(100.0 - idle))
}
