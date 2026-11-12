//! The Linux readers on the machine that runs the tests: read only (MET-002).

use agent_core::points::samples;
use anima_agent::readers::{Platform, reading};
use chrono::Utc;

#[tokio::test]
async fn met_002_a_linux_reading_has_cpu_memory_load_uptime_and_disks() {
    let first = reading(Platform::Linux, Utc::now()).await;
    assert!(first.cpu.is_some());
    assert!(first.memory.is_some_and(|memory| memory.total_bytes > 0));
    assert!(first.load.is_some());
    assert!(first.uptime_seconds.is_some());
    assert!(!first.disks.is_empty(), "df -P -B1 listed no disk");
    assert!(first.interfaces.iter().any(|one| !one.name.is_empty()));
    let second = reading(Platform::Linux, Utc::now()).await;
    let made = samples(Some(&first), &second, std::time::Duration::from_secs(2));
    assert!(
        made.iter()
            .any(|sample| sample.kind.as_str() == "memory_percent")
    );
}

#[tokio::test]
async fn met_006_amd_cards_are_numbered_by_their_drm_card_number() {
    let drm = tempfile::tempdir().expect("a folder");
    let make = |card: &str, vendor: &str, busy: Option<&str>| {
        let device = drm.path().join(card).join("device");
        std::fs::create_dir_all(&device).expect("dirs");
        std::fs::write(device.join("vendor"), format!("{vendor}\n")).expect("vendor");
        if let Some(busy) = busy {
            std::fs::write(device.join("gpu_busy_percent"), format!("{busy}\n")).expect("busy");
        }
    };
    make("card0", "0x8086", Some("5"));
    make("card2", "0x1002", Some("37"));
    make("card7", "0x1002", Some("12"));
    make("card9", "0x1002", None);
    std::fs::create_dir_all(drm.path().join("card1-HDMI-A-1")).expect("a connector");
    let cards = anima_agent::readers::gpu::amd_cards(drm.path()).await;
    let seen: Vec<(u32, Option<f64>)> = cards
        .iter()
        .map(|card| (card.index, card.busy_percent))
        .collect();
    assert_eq!(seen, [(2, Some(37.0)), (7, Some(12.0))]);
}

#[tokio::test]
async fn met_022_a_linux_reading_lists_drives_and_their_counters() {
    let read = reading(Platform::Linux, Utc::now()).await;
    let Some(drives) = read.drives else {
        // A container without lsblk or /sys/block reads no drives; that is not a failure.
        return;
    };
    for drive in &drives {
        assert!(drive.size_bytes > 0);
        assert!(drive.mounts.iter().all(|mount| mount.starts_with('/')));
    }
    assert!(
        !read.drive_counters.is_empty(),
        "/proc/diskstats listed nothing"
    );
}
