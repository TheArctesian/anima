//! The commands a collection runs on a device, by operating system (MET-002, MET-005, CHK-012,
//! D-3). Pure.
//!
//! Every word is static text written here: no device field, tag or other operator text can
//! reach a command line. Each command only reads (`cat` of `/proc` files and of `/etc/os-release`,
//! `df`, `sysctl -n`, `vm_stat`, `netstat`, `top` in logging mode, and the GPU queries of spec
//! 6.6: `nvidia-smi` queries, `grep` of `amdgpu` sysfs files, `intel_gpu_top` for 2 seconds,
//! `ioreg`); none writes, starts a service or takes input.

use std::time::Duration;

use crate::{devices::OperatingSystem, metrics::GpuVendor, ssh::FixedCommand};

/// The limit of a command that prints a file or a table at once.
const QUICK: Duration = Duration::from_secs(5);
/// The limit of `top -l 2 -s 1`, which waits one second between its two samples.
const TOP: Duration = Duration::from_secs(8);
/// The limit of the `intel_gpu_top` line: 2 seconds of samples, 1 more for `timeout` to kill a
/// tool that ignores the interrupt, and a margin.
const INTEL: Duration = Duration::from_secs(4);

/// Which output a command gives, so its reader is known (see [`agent_core::readings`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// Linux `cat /proc/stat`.
    Stat,
    /// Linux `cat /proc/meminfo`.
    Meminfo,
    /// Linux `cat /proc/net/dev`.
    NetDev,
    /// Linux `cat /proc/uptime`.
    Uptime,
    /// Linux `cat /proc/loadavg`.
    Loadavg,
    /// Linux `df -P -B1`: blocks of 1 byte.
    DiskBytes,
    /// Linux `cat /etc/os-release`: the distribution, not a metric (DEV-016).
    OsRelease,
    /// macOS `sysctl -n hw.memsize hw.pagesize vm.loadavg kern.boottime`.
    Sysctl,
    /// macOS `vm_stat`.
    VmStat,
    /// macOS `df -P -k`: blocks of 1024 bytes.
    DiskKilobytes,
    /// macOS `netstat -ib -n`.
    Netstat,
    /// macOS `top -l 2 -n 0 -s 1`.
    Top,
}

/// One command of a collection with the output it gives and its time limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Command {
    pub(super) source: Source,
    pub(super) fixed: FixedCommand,
    pub(super) limit: Duration,
}

/// The commands for a device running `os`, in the order they run. Windows and other systems
/// get none: they are not collected over SSH (MET-005).
pub(super) fn commands(os: OperatingSystem) -> Vec<Command> {
    match os {
        OperatingSystem::Linux => linux(),
        OperatingSystem::Macos => macos(),
        OperatingSystem::Windows | OperatingSystem::Other => Vec::new(),
    }
}

fn linux() -> Vec<Command> {
    vec![
        quick(Source::Stat, FixedCommand::new("cat").arg("/proc/stat")),
        quick(
            Source::Meminfo,
            FixedCommand::new("cat").arg("/proc/meminfo"),
        ),
        quick(
            Source::NetDev,
            FixedCommand::new("cat").arg("/proc/net/dev"),
        ),
        quick(Source::Uptime, FixedCommand::new("cat").arg("/proc/uptime")),
        quick(
            Source::Loadavg,
            FixedCommand::new("cat").arg("/proc/loadavg"),
        ),
        quick(
            Source::DiskBytes,
            FixedCommand::new("df").arg("-P").arg("-B1"),
        ),
        quick(
            Source::OsRelease,
            FixedCommand::new("cat").arg("/etc/os-release"),
        ),
    ]
}

fn macos() -> Vec<Command> {
    let sysctl = FixedCommand::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .arg("hw.pagesize")
        .arg("vm.loadavg")
        .arg("kern.boottime");
    let top = FixedCommand::new("top")
        .arg("-l")
        .arg("2")
        .arg("-n")
        .arg("0")
        .arg("-s")
        .arg("1");
    vec![
        quick(Source::Sysctl, sysctl),
        quick(Source::VmStat, FixedCommand::new("vm_stat")),
        quick(
            Source::DiskKilobytes,
            FixedCommand::new("df").arg("-P").arg("-k"),
        ),
        quick(
            Source::Netstat,
            FixedCommand::new("netstat").arg("-ib").arg("-n"),
        ),
        Command {
            source: Source::Top,
            fixed: top,
            limit: TOP,
        },
    ]
}

fn quick(source: Source, fixed: FixedCommand) -> Command {
    Command {
        source,
        fixed,
        limit: QUICK,
    }
}

/// One GPU vendor's read, the vendor it asks about and its time limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GpuCommand {
    pub(super) vendor: GpuVendor,
    pub(super) fixed: FixedCommand,
    pub(super) limit: Duration,
}

/// The GPU reads for a device running `os`, one per vendor it can have, in the order they run
/// (spec 6.6, D-30). A missing tool exits 127, which reads as no GPU of that vendor; another
/// non-zero exit is a failed read, which discovery counts by the GPUs known (MET-006). Windows
/// and other systems get none (MET-005). Reading a GPU's values can wake it from runtime power
/// saving (the driver does that for `nvidia-smi` and the `amdgpu` files); that is a side
/// effect of the read, not a write (MET-002).
pub(super) fn gpu_commands(os: OperatingSystem) -> Vec<GpuCommand> {
    match os {
        OperatingSystem::Linux => vec![
            gpu(GpuVendor::Nvidia, nvidia_smi(), QUICK),
            gpu(GpuVendor::Amd, amd_sysfs(), QUICK),
            gpu(GpuVendor::Intel, intel_gpu_top(), INTEL),
        ],
        OperatingSystem::Macos => vec![gpu(GpuVendor::Apple, ioreg(), QUICK)],
        OperatingSystem::Windows | OperatingSystem::Other => Vec::new(),
    }
}

/// The spec 6.6 query: one CSV line per GPU, memory in MiB.
fn nvidia_smi() -> FixedCommand {
    FixedCommand::new("nvidia-smi")
        .arg(concat!(
            "--query-gpu=index,name,utilization.gpu,memory.used,memory.total,",
            "temperature.gpu,power.draw"
        ))
        .arg("--format=csv,noheader,nounits")
}

/// The three `amdgpu` files of every card, each line `<path>:<value>` (`-H`), so each card's
/// values stay apart. Three plain globs: the device's shell may be `dash`, which has no brace
/// expansion. A glob that matches nothing makes `grep` exit 2 with the other files printed.
fn amd_sysfs() -> FixedCommand {
    FixedCommand::new("grep")
        .arg("-H")
        .arg(".")
        .arg("/sys/class/drm/card*/device/gpu_busy_percent")
        .arg("/sys/class/drm/card*/device/mem_info_vram_used")
        .arg("/sys/class/drm/card*/device/mem_info_vram_total")
}

/// `intel_gpu_top -J` sampling every 500 ms, interrupted after 2 seconds and killed 1 second
/// later if it is still running. `--preserve-status` gives the tool's own status: non-zero
/// when it may not read the GPU counters (D-30), and non-zero from a `timeout` without these
/// options. Unproven: that the tool ends its JSON array and exits 0 on the interrupt; no Intel
/// device was at hand. If it does not, the read exits non-zero and no Intel GPU is found,
/// never a failed collection.
fn intel_gpu_top() -> FixedCommand {
    FixedCommand::new("timeout")
        .arg("--preserve-status")
        .arg("-s")
        .arg("INT")
        .arg("-k")
        .arg("1")
        .arg("2")
        .arg("intel_gpu_top")
        .arg("-J")
        .arg("-s")
        .arg("500")
}

/// The accelerators' registry entries, one level deep: usage only over SSH (spec 6.6).
fn ioreg() -> FixedCommand {
    FixedCommand::new("ioreg")
        .arg("-r")
        .arg("-d")
        .arg("1")
        .arg("-c")
        .arg("IOAccelerator")
}

fn gpu(vendor: GpuVendor, fixed: FixedCommand, limit: Duration) -> GpuCommand {
    GpuCommand {
        vendor,
        fixed,
        limit,
    }
}
