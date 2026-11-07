//! The exact commands per operating system: static, read-only, the forms the readers parse
//! (MET-002, MET-005, CHK-012, D-3).

use std::time::Duration;

use crate::{
    devices::OperatingSystem,
    metrics::{
        collector::DEADLINE,
        commands::{Command, Source, commands},
    },
    sessions::FixedCommand,
};

/// The command of `words`, built here word by word.
fn fixed(words: &[&'static str]) -> FixedCommand {
    let (program, args) = words.split_first().unwrap();
    args.iter()
        .fold(FixedCommand::new(program), |command, arg| command.arg(arg))
}

fn command(source: Source, words: &[&'static str], seconds: u64) -> Command {
    Command {
        source,
        fixed: fixed(words),
        limit: Duration::from_secs(seconds),
    }
}

#[test]
fn met_002_linux_runs_six_cat_and_df_commands_in_order() {
    assert_eq!(
        commands(OperatingSystem::Linux),
        [
            command(Source::Stat, &["cat", "/proc/stat"], 5),
            command(Source::Meminfo, &["cat", "/proc/meminfo"], 5),
            command(Source::NetDev, &["cat", "/proc/net/dev"], 5),
            command(Source::Uptime, &["cat", "/proc/uptime"], 5),
            command(Source::Loadavg, &["cat", "/proc/loadavg"], 5),
            command(Source::DiskBytes, &["df", "-P", "-B1"], 5),
        ]
    );
}

#[test]
fn met_002_macos_runs_sysctl_vm_stat_df_netstat_and_top_in_order() {
    let sysctl = [
        "sysctl",
        "-n",
        "hw.memsize",
        "hw.pagesize",
        "vm.loadavg",
        "kern.boottime",
    ];
    assert_eq!(
        commands(OperatingSystem::Macos),
        [
            command(Source::Sysctl, &sysctl, 5),
            command(Source::VmStat, &["vm_stat"], 5),
            command(Source::DiskKilobytes, &["df", "-P", "-k"], 5),
            command(Source::Netstat, &["netstat", "-ib", "-n"], 5),
            command(Source::Top, &["top", "-l", "2", "-n", "0", "-s", "1"], 8),
        ]
    );
}

#[test]
fn met_005_windows_and_other_systems_run_nothing() {
    assert_eq!(commands(OperatingSystem::Windows), []);
    assert_eq!(commands(OperatingSystem::Other), []);
}

#[test]
fn d11_every_command_fits_in_a_collection() {
    for os in [OperatingSystem::Linux, OperatingSystem::Macos] {
        for command in commands(os) {
            assert!(command.limit < DEADLINE, "{command:?}");
        }
    }
}
