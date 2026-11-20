//! The reader of the container of a hosted hub, on files that a test lays out (HMD-071 to HMD-074).

use std::path::Path;
use std::time::{Duration, Instant};

use anima_daemon::readers::{Source, container::Container};
use chrono::Utc;
use daemon_core::points::samples;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The files of a server beside those of the container of a hub, and 123 bytes of data of the
/// hub with a link out of its folder that must not count.
fn lay_out(root: &Path) {
    // The server: 64 GiB of memory, a load, its interfaces and its time since boot.
    let meminfo = "MemTotal: 67108864 kB\nMemAvailable: 33554432 kB\n";
    write(root, "proc/meminfo", meminfo);
    write(root, "proc/loadavg", "7.50 6.00 5.00 3/900 4242\n");
    write(root, "proc/uptime", "9000000.00 1.00\n");
    let net = "Inter-|\n face |\n  eth0: 999999 9 0 0 0 0 0 0 888888 8 0 0 0 0 0 0\n";
    write(root, "proc/net/dev", net);
    write(root, "proc/stat", "cpu 100 0 100 100000 0 0 0 0 0 0\n");
    // The container: 1.5 GiB of memory, one processor with no quota, its own processor time.
    write(root, "sys/fs/cgroup/memory.max", "1610612736\n");
    write(root, "sys/fs/cgroup/memory.current", "359116800\n");
    let stat = "anon 99278848\ninactive_file 159383552\n";
    write(root, "sys/fs/cgroup/memory.stat", stat);
    write(root, "sys/fs/cgroup/cpu.max", "max 100000\n");
    write(root, "sys/fs/cgroup/cpu.stat", "usage_usec 412718333\n");
    write(root, "var/lib/anima/db/base", &"x".repeat(100));
    write(root, "var/lib/anima/files/one", &"y".repeat(23));
    let out = root.join("var/lib/anima/out");
    std::os::unix::fs::symlink(root.join("proc/meminfo"), out).unwrap();
}

/// HMD-074: the daemon of a hosted hub is given the files of a server beside those of its
/// container, and no figure of the server is in what it reads or sends.
#[tokio::test]
async fn hmd_074_a_hosted_hub_reads_its_container_and_nothing_of_the_server() {
    let root = tempfile::tempdir().unwrap();
    lay_out(root.path());
    let data = root.path().join("var/lib/anima");
    let reader = Container::new(root.path().to_path_buf(), data.clone(), 1);
    let mut source = Source::HostedHub(reader);
    let start = Instant::now();
    let first = source.reading(Utc::now(), start).await;
    write(
        root.path(),
        "sys/fs/cgroup/cpu.stat",
        "usage_usec 415218333\n",
    );
    let later = start + Duration::from_secs(10);
    let second = source.reading(Utc::now(), later).await;

    let memory = second.memory.unwrap();
    assert_eq!(memory.total_bytes, 1_610_612_736);
    assert_eq!(memory.used_bytes, 359_116_800 - 159_383_552);
    assert!(second.load.is_none() && second.uptime_seconds.is_none());
    assert!(second.disks.is_empty() && second.interfaces.is_empty());
    assert!(second.drives.is_none() && second.drive_counters.is_empty());
    let kept = second.hub_storage.as_ref().unwrap();
    let expected = (data.to_str().unwrap(), 123);
    assert_eq!((kept.folder.as_str(), kept.bytes), expected);

    let made = samples(Some(&first), &second, Duration::from_secs(10));
    let kinds: Vec<&str> = made.iter().map(|sample| sample.kind.as_str()).collect();
    let only = [
        "cpu_percent",
        "memory_percent",
        "memory_bytes",
        "disk_bytes",
    ];
    assert_eq!(kinds, only);
    // 2.5 seconds of one processor in 10 seconds.
    assert!((made[0].value - 25.0).abs() < 1e-6);
    let server = [64.0 * 1024.0 * 1024.0 * 1024.0, 7.5, 9_000_000.0, 999_999.0];
    assert!(made.iter().all(|sample| !server.contains(&sample.value)));
}
