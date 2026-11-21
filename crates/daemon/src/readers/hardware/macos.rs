//! The macOS hardware values: `sysctl`, `pmset` and `ioreg` with fixed arguments (CAP-001,
//! CAP-004, CAP-005). The usable memory of Apple silicon is `iogpu.wired_limit_mb` when the
//! operator set it, else the default share of macOS (`daemon_core::hardware::parse`).

use std::time::Duration;

use daemon_core::hardware::parse::{ioreg_active, pmset_thermal, sysctl_number, usable_memory};
use daemon_core::hardware::power::from_pmset;
use daemon_core::hardware::{Facts, Hardware, KfdAccess, PowerSource, backends};
use daemon_core::inventory::Os;
use daemon_core::readings::GpuVendor;

use super::{FixedCommand, cpu_features, free_disk_bytes, output};

const fn fixed(program: &'static str, args: &'static [&'static str]) -> FixedCommand {
    FixedCommand {
        program,
        args,
        limit: Duration::from_secs(2),
        keep_cut_output: false,
    }
}

const MEMSIZE: FixedCommand = fixed("/usr/sbin/sysctl", &["-n", "hw.memsize"]);
/// Fails on an Intel Mac, which has no `iogpu`.
const WIRED_LIMIT: FixedCommand = fixed("/usr/sbin/sysctl", &["-n", "iogpu.wired_limit_mb"]);
const BATTERY: FixedCommand = fixed("/usr/bin/pmset", &["-g", "batt"]);
const THERMAL: FixedCommand = fixed("/usr/bin/pmset", &["-g", "therm"]);
const IDLE: FixedCommand = fixed(
    "/usr/sbin/ioreg",
    &["-c", "IOHIDSystem", "-d", "4", "-r", "-k", "HIDIdleTime"],
);

pub(super) async fn read() -> Option<Hardware> {
    let (memsize, wired, battery, thermal, idle) = tokio::join!(
        output(&MEMSIZE),
        output(&WIRED_LIMIT),
        output(&BATTERY),
        output(&THERMAL),
        output(&IDLE),
    );
    let memory = sysctl_number(&memsize?)?;
    let apple_silicon = std::env::consts::ARCH == "aarch64";
    let facts = Facts {
        os: Os::Macos,
        apple_silicon,
        // Each Intel Mac that macOS 27 runs on has a Metal GPU.
        vendors: if apple_silicon {
            vec![GpuVendor::Apple]
        } else {
            vec![GpuVendor::Intel]
        },
        nvidia_driver: false,
        kfd: KfdAccess::Missing,
        vulkan_driver: false,
    };
    let wired = wired.and_then(|bytes| sysctl_number(&bytes));
    Some(Hardware {
        system_memory_bytes: memory,
        unified_memory_bytes: apple_silicon.then_some(memory),
        dedicated_memory_bytes: None,
        usable_memory: apple_silicon.then(|| usable_memory(memory, wired)),
        free_disk_bytes: free_disk_bytes(),
        cpu_features: cpu_features(),
        backends: backends(&facts),
        power_source: battery
            .map(|bytes| PowerSource::from(from_pmset(&String::from_utf8_lossy(&bytes)))),
        thermal_state: thermal.and_then(|bytes| pmset_thermal(&bytes)),
        operator_active: idle.and_then(|bytes| ioreg_active(&bytes)),
    })
}
