//! The exact commands per operating system, the GPU reads included: static, read-only, the
//! forms the readers parse (MET-002, MET-005, CHK-012, D-3, D-30).

use std::time::Duration;

use crate::{
    devices::OperatingSystem,
    metrics::{
        GpuVendor,
        collector::{DEADLINE, GPU_WINDOW},
        commands::{Command, Source, commands, gpu_commands},
    },
    ssh::FixedCommand,
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
fn met_002_linux_runs_the_cat_df_and_lsblk_commands_then_reads_os_release_in_order() {
    let lsblk = |columns| ["lsblk", "-J", "-b", "-o", columns];
    assert_eq!(
        commands(OperatingSystem::Linux),
        [
            command(Source::Stat, &["cat", "/proc/stat"], 5),
            command(Source::Meminfo, &["cat", "/proc/meminfo"], 5),
            command(Source::NetDev, &["cat", "/proc/net/dev"], 5),
            command(Source::Uptime, &["cat", "/proc/uptime"], 5),
            command(Source::Loadavg, &["cat", "/proc/loadavg"], 5),
            command(Source::DiskBytes, &["df", "-P", "-B1"], 5),
            command(Source::Diskstats, &["cat", "/proc/diskstats"], 5),
            command(
                Source::Drives,
                &lsblk("NAME,TYPE,SIZE,ROTA,TRAN,MOUNTPOINTS"),
                5
            ),
            command(
                Source::DrivesOld,
                &lsblk("NAME,TYPE,SIZE,ROTA,TRAN,MOUNTPOINT"),
                5
            ),
            command(Source::OsRelease, &["cat", "/etc/os-release"], 5),
        ]
    );
}

#[test]
fn met_027_only_the_distribution_and_the_drive_reads_may_fail_without_ending_a_collection() {
    let optional: Vec<Source> = commands(OperatingSystem::Linux)
        .into_iter()
        .chain(commands(OperatingSystem::Macos))
        .map(|command| command.source)
        .filter(|source| source.is_optional())
        .collect();
    assert_eq!(
        optional,
        [
            Source::Diskstats,
            Source::Drives,
            Source::DrivesOld,
            Source::OsRelease
        ]
    );
}

#[test]
fn dev_016_only_linux_reads_os_release() {
    let reads = |os| {
        commands(os)
            .iter()
            .any(|command| command.source == Source::OsRelease)
    };
    assert!(reads(OperatingSystem::Linux));
    for os in [
        OperatingSystem::Macos,
        OperatingSystem::Windows,
        OperatingSystem::Other,
    ] {
        assert!(!reads(os), "{os:?}");
    }
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

#[test]
fn chk_012_gpu_reads_are_the_static_spec_forms_per_system() {
    let linux: Vec<_> = gpu_commands(OperatingSystem::Linux)
        .into_iter()
        .map(|command| (command.vendor, command.fixed, command.limit))
        .collect();
    let nvidia = fixed(&[
        "nvidia-smi",
        concat!(
            "--query-gpu=index,name,utilization.gpu,memory.used,memory.total,",
            "temperature.gpu,power.draw"
        ),
        "--format=csv,noheader,nounits",
    ]);
    let amd = fixed(&[
        "grep",
        "-H",
        ".",
        "/sys/class/drm/card*/device/gpu_busy_percent",
        "/sys/class/drm/card*/device/mem_info_vram_used",
        "/sys/class/drm/card*/device/mem_info_vram_total",
    ]);
    let intel = fixed(&[
        "timeout",
        "--preserve-status",
        "-s",
        "INT",
        "-k",
        "1",
        "2",
        "intel_gpu_top",
        "-J",
        "-s",
        "500",
    ]);
    let seconds = Duration::from_secs;
    assert_eq!(
        linux,
        [
            (GpuVendor::Nvidia, nvidia, seconds(5)),
            (GpuVendor::Amd, amd, seconds(5)),
            (GpuVendor::Intel, intel, seconds(4)),
        ]
    );
    let macos: Vec<_> = gpu_commands(OperatingSystem::Macos)
        .into_iter()
        .map(|command| (command.vendor, command.fixed))
        .collect();
    let ioreg = fixed(&["ioreg", "-r", "-d", "1", "-c", "IOAccelerator"]);
    assert_eq!(macos, [(GpuVendor::Apple, ioreg)]);
    assert_eq!(gpu_commands(OperatingSystem::Windows), []);
    assert_eq!(gpu_commands(OperatingSystem::Other), []);
}

#[test]
fn d11_every_gpu_read_fits_in_the_gpu_window_inside_the_deadline() {
    assert!(GPU_WINDOW < DEADLINE);
    for os in [OperatingSystem::Linux, OperatingSystem::Macos] {
        for command in gpu_commands(os) {
            assert!(command.limit < GPU_WINDOW, "{command:?}");
        }
    }
}
