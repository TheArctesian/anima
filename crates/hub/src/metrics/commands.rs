//! The commands a collection runs on a device, by operating system (MET-002, MET-005, CHK-012,
//! D-3). Pure.
//!
//! Every word is static text written here: no device field, tag or other operator text can
//! reach a command line. Each command only reads (`cat` of `/proc` files and of
//! `/etc/os-release`, `df`, `sysctl -n`,
//! `vm_stat`, `netstat`, `top` in logging mode); none writes, starts a service or takes input.

use std::time::Duration;

use crate::{devices::OperatingSystem, ssh::FixedCommand};

/// The limit of a command that prints a file or a table at once.
const QUICK: Duration = Duration::from_secs(5);
/// The limit of `top -l 2 -s 1`, which waits one second between its two samples.
const TOP: Duration = Duration::from_secs(8);

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
