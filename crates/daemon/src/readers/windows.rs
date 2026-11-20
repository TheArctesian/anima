//! Native Windows counters. Missing OS concepts (Unix load averages) remain absent.
//! Keep CPU history across samples; the first sample has no fabricated CPU percentage.
use daemon_core::readings::{Cpu, Disk, Interface, Memory, Reading};
use std::sync::{Mutex, OnceLock};
use sysinfo::{CpuRefreshKind, Disks, Networks, RefreshKind, System};

static SYSTEM: OnceLock<Mutex<Option<System>>> = OnceLock::new();

pub async fn reading() -> Reading {
    tokio::task::spawn_blocking(collect)
        .await
        .unwrap_or_default()
}

fn collect() -> Reading {
    let store = SYSTEM.get_or_init(|| Mutex::new(None));
    let Ok(mut store) = store.lock() else {
        return Reading::default();
    };
    let first = store.is_none();
    if first {
        *store = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_usage()),
        )
        .ok();
    }
    let Some(system) = store.as_mut() else {
        return Reading::default();
    };
    system.refresh_memory();
    if !first {
        system.refresh_cpu_usage();
    }
    let total_bytes = system.total_memory();
    Reading {
        cpu: (!first).then(|| Cpu::Percent(f64::from(system.global_cpu_usage()))),
        memory: (total_bytes > 0).then(|| Memory {
            total_bytes,
            used_bytes: system.used_memory().min(total_bytes),
        }),
        uptime_seconds: System::uptime()
            .ok()
            .and_then(|seconds| u32::try_from(seconds).ok())
            .map(f64::from),
        disks: disks(),
        interfaces: interfaces(),
        ..Reading::default()
    }
}

fn label(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && !text
            .chars()
            .any(|c| c.is_control() || c == char::REPLACEMENT_CHARACTER)
}

fn disks() -> Vec<Disk> {
    let Ok(disks) = Disks::new_with_refreshed_list() else {
        return Vec::new();
    };
    disks
        .iter()
        .filter_map(|disk| {
            let mount = disk.mount_point().to_str()?;
            let filesystem = disk.file_system().to_str()?;
            if !label(mount) || !label(filesystem) {
                return None;
            }
            Some(Disk {
                filesystem: filesystem.to_owned(),
                mount: mount.to_owned(),
                total_bytes: disk.total_space(),
                used_bytes: disk.total_space().saturating_sub(disk.available_space()),
                available_bytes: disk.available_space(),
            })
        })
        .collect()
}

fn interfaces() -> Vec<Interface> {
    let Ok(networks) = Networks::new_with_refreshed_list() else {
        return Vec::new();
    };
    networks
        .iter()
        .filter_map(|(name, counters)| {
            let name = name.to_str().filter(|name| label(name))?;
            Some(Interface {
                name: name.to_owned(),
                received_bytes: counters.total_received(),
                sent_bytes: counters.total_transmitted(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_reports_real_memory_mounts_and_cpu_samples() {
        let first = collect();
        assert!(first.memory.is_some_and(
            |memory| memory.total_bytes > 0 && memory.used_bytes <= memory.total_bytes
        ));
        assert!(first.uptime_seconds.is_some_and(|uptime| uptime >= 0.0));
        assert_ne!(first.disks.len(), 0);
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        assert!(
            matches!(collect().cpu, Some(Cpu::Percent(value)) if (0.0..=100.0).contains(&value))
        );
        assert!(first.load.is_none());
    }
}
