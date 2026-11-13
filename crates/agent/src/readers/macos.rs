//! The macOS collection: `sysctl`, `vm_stat`, `df -P -k`, `netstat -ib -n` and `top -l 2`
//! (MET-002). The commands are the forms in `agent_core::readings` (MET-016).
//!
//! Not built on macOS yet (R-092 D-8); it runs only fixed commands, so it compiles anywhere.

use std::time::Duration;

use agent_core::readings::{Memory, Reading, df, macos};
use chrono::{DateTime, Utc};

use super::command::FixedCommand;

const fn fixed(program: &'static str, args: &'static [&'static str], seconds: u64) -> FixedCommand {
    FixedCommand {
        program,
        args,
        limit: Duration::from_secs(seconds),
        keep_cut_output: false,
    }
}

const SYSCTL: FixedCommand = fixed(
    "/usr/sbin/sysctl",
    &[
        "-n",
        "hw.memsize",
        "hw.pagesize",
        "vm.loadavg",
        "kern.boottime",
    ],
    1,
);
const VM_STAT: FixedCommand = fixed("/usr/bin/vm_stat", &[], 1);
const DF: FixedCommand = fixed("/bin/df", &["-P", "-k"], 2);
const NETSTAT: FixedCommand = fixed("/usr/sbin/netstat", &["-ib", "-n"], 2);
/// Two samples a second apart: the first is the average since boot (see `macos::cpu`).
const TOP: FixedCommand = fixed("/usr/bin/top", &["-l", "2", "-n", "0", "-s", "1"], 5);

pub async fn reading(now: DateTime<Utc>) -> Reading {
    let (sysctl, vm_stat, df_out, netstat, top) = tokio::join!(
        output(&SYSCTL),
        output(&VM_STAT),
        output(&DF),
        output(&NETSTAT),
        output(&TOP),
    );
    let sysctl = sysctl.map_or_default(|bytes| macos::sysctl(&bytes));
    let used = vm_stat.and_then(|bytes| macos::memory_used_bytes(&bytes, sysctl.page_size));
    Reading {
        cpu: top.and_then(|bytes| macos::cpu(&bytes)),
        memory: sysctl
            .memory_bytes
            .zip(used)
            .map(|(total_bytes, used_bytes)| Memory {
                total_bytes,
                used_bytes,
            }),
        load: sysctl.load,
        uptime_seconds: sysctl.uptime_seconds(now),
        disks: df_out.map_or_default(|bytes| df::disks(&bytes, 1024)),
        interfaces: netstat.map_or_default(|bytes| macos::interfaces(&bytes)),
        // No `lsblk` on macOS: its mounts stand alone (MET-031).
        drives: None,
        drive_counters: Vec::new(),
    }
}

async fn output(command: &FixedCommand) -> Option<Vec<u8>> {
    match command.run().await {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            tracing::debug!(%error, "a macOS reader gave no reading");
            None
        }
    }
}
