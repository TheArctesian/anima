//! From readings to samples: shares, rates and the plain values (MET-002, D-4, D-5).

use std::time::Duration;

use super::{
    Kind, Sample,
    labels::{kept_disks, kept_interfaces},
};
use crate::readings::{Cpu, Interface, Reading};

/// The samples of `current`. `previous` is the reading before it and `elapsed` the time between
/// the two. With no `previous`, a counter that went back (a reboot, an interface reset) or no
/// time passed, the Linux CPU share and the network rates are left out (D-4). The macOS CPU share
/// needs no earlier reading, so it is always given. Disks and interfaces follow
/// [`MAX_LABELS`](super::MAX_LABELS) and the exclusions of D-5.
#[must_use]
pub fn samples(previous: Option<&Reading>, current: &Reading, elapsed: Duration) -> Vec<Sample> {
    let mut out = Vec::new();
    let before = previous.and_then(|reading| reading.cpu);
    let cpu = cpu_percent(before, current.cpu, elapsed);
    out.extend(cpu.and_then(|value| Sample::new(Kind::CpuPercent, "", value)));
    if let Some(memory) = current.memory.filter(|memory| memory.total_bytes > 0) {
        let used = memory.used_bytes.min(memory.total_bytes);
        let share = float(used) / float(memory.total_bytes) * 100.0;
        out.extend(Sample::new(Kind::MemoryPercent, "", share));
        out.extend(Sample::new(Kind::MemoryBytes, "", float(used)));
    }
    out.extend(
        current
            .uptime_seconds
            .and_then(|value| Sample::new(Kind::UptimeSeconds, "", value)),
    );
    if let Some(load) = current.load {
        out.extend(Sample::new(Kind::Load1, "", load.one));
        out.extend(Sample::new(Kind::Load5, "", load.five));
        out.extend(Sample::new(Kind::Load15, "", load.fifteen));
    }
    for disk in kept_disks(&current.disks) {
        if disk.total_bytes == 0 {
            continue;
        }
        let used = disk.used_bytes.min(disk.total_bytes);
        // `df`'s Capacity: used over used plus available, which is what macOS and Linux show.
        let size = used.saturating_add(disk.available_bytes);
        if size == 0 {
            continue;
        }
        let share = float(used) / float(size) * 100.0;
        out.extend(Sample::new(Kind::DiskPercent, &disk.mount, share));
        out.extend(Sample::new(Kind::DiskBytes, &disk.mount, float(used)));
    }
    if let Some(previous) = previous {
        out.extend(network(previous, current, elapsed));
    }
    out
}

fn cpu_percent(previous: Option<Cpu>, current: Option<Cpu>, elapsed: Duration) -> Option<f64> {
    match (previous, current?) {
        (_, Cpu::Percent(share)) => Some(share),
        (
            Some(Cpu::Ticks {
                idle: was_idle,
                total: was_total,
            }),
            Cpu::Ticks { idle, total },
        ) if !elapsed.is_zero() => {
            let total_delta = total.checked_sub(was_total).filter(|delta| *delta > 0)?;
            let idle_delta = idle.checked_sub(was_idle)?;
            let busy_delta = total_delta.checked_sub(idle_delta)?;
            Some(float(busy_delta) / float(total_delta) * 100.0)
        }
        _ => None,
    }
}

fn network(previous: &Reading, current: &Reading, elapsed: Duration) -> Vec<Sample> {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for now in kept_interfaces(&current.interfaces) {
        let Some(was) = earlier(&previous.interfaces, now) else {
            continue;
        };
        let received = now.received_bytes.checked_sub(was.received_bytes);
        let sent = now.sent_bytes.checked_sub(was.sent_bytes);
        let (Some(received), Some(sent)) = (received, sent) else {
            continue;
        };
        out.extend(Sample::new(
            Kind::NetworkInBps,
            &now.name,
            float(received) / seconds,
        ));
        out.extend(Sample::new(
            Kind::NetworkOutBps,
            &now.name,
            float(sent) / seconds,
        ));
    }
    out
}

/// The reading of the same interface before (the first, if a name repeats).
fn earlier<'a>(before: &'a [Interface], now: &Interface) -> Option<&'a Interface> {
    before.iter().find(|one| one.name == now.name)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counters above 2^53 lose their last digits, which a chart cannot show"
)]
const fn float(count: u64) -> f64 {
    count as f64
}
