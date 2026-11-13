//! Rounds against the test SSH device with the pop-os `lsblk` and `/proc/diskstats` (MET-027,
//! MET-028, MET-029).

use agent_core::readings::captures::{LSBLK_OLD, PROC_DISKSTATS};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::super::bench::{DOCKER_HOST, LINUX_LINES, LSBLK_LINE, LSBLK_OLD_LINE, bench, device};
use crate::{
    clock::Clock as _,
    fleet::metrics::{DriveKind, MetricKind, MetricsAdded, storage},
};

/// The stored drives of `docker-host`: name, kind, size, mounts and when they were found.
async fn stored(pool: &PgPool) -> Vec<(String, String, i64, Vec<String>, DateTime<Utc>)> {
    sqlx::query_as(
        "SELECT name, kind, size_bytes, mounts, found_at FROM metrics.drives \
         WHERE device_id = $1 ORDER BY name",
    )
    .bind(device(DOCKER_HOST))
    .fetch_all(pool)
    .await
    .unwrap()
}

fn value(added: &MetricsAdded, kind: MetricKind, label: &str) -> Option<f64> {
    added
        .samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_027_the_first_round_stores_the_drives_and_an_unchanged_list_is_not_written_again(
    pool: PgPool,
) {
    let mut bench = bench(&pool, true).await;
    bench.collector.round().await;
    let first = stored(&pool).await;
    let names: Vec<&str> = first.iter().map(|row| row.0.as_str()).collect();
    assert_eq!(names, ["nvme0n1", "nvme1n1", "sda", "sdb"]);
    assert_eq!(first[1].1, "nvme");
    assert_eq!(first[1].2, 1_000_204_886_016);
    assert_eq!(first[1].3, ["/", "/boot/efi"]);
    assert_eq!(first[2].1, "hdd");
    assert!(first[2].3.is_empty(), "sda is not mounted");

    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(
        stored(&pool).await,
        first,
        "the same list is not written again"
    );

    let mut changed = String::from_utf8(LSBLK_POP_OS_TEXT.to_vec()).unwrap();
    changed = changed.replacen(
        r#""mountpoints":[null],"#,
        r#""mountpoints":["/mnt/archive"],"#,
        1,
    );
    bench.server.answer(LSBLK_LINE, changed.as_bytes());
    bench.clock.advance(30);
    bench.collector.round().await;
    let now = stored(&pool).await;
    assert_eq!(now[2].3, ["/mnt/archive"], "sda is mounted now");
    assert!(now[2].4 > first[2].4, "found again");
}

/// The pop-os capture, for a test that changes it.
const LSBLK_POP_OS_TEXT: &[u8] = agent_core::readings::captures::LSBLK_POP_OS;

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_027_a_failed_lsblk_keeps_the_stored_drives_and_fails_nothing(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.collector.round().await;
    let first = stored(&pool).await;
    bench.server.answers.lock().unwrap().remove(LSBLK_LINE);
    bench
        .server
        .answers
        .lock()
        .unwrap()
        .remove("cat /proc/diskstats");
    bench.clock.advance(30);
    bench.collector.round().await;
    assert_eq!(stored(&pool).await, first);
    bench.feed.try_recv().unwrap();
    let second = bench.feed.try_recv().unwrap();
    assert!(
        value(&second, MetricKind::MemoryPercent, "").is_some(),
        "collected"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_027_old_util_linux_is_read_with_the_mountpoint_column(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.server.answers.lock().unwrap().remove(LSBLK_LINE);
    bench.server.answer(LSBLK_OLD_LINE, LSBLK_OLD);
    bench.collector.round().await;
    let heard = bench.server.heard();
    let lsblk: Vec<&str> = heard
        .iter()
        .map(String::as_str)
        .filter(|line| line.starts_with("lsblk"))
        .collect();
    assert_eq!(lsblk, [LSBLK_LINE, LSBLK_OLD_LINE]);
    let names: Vec<String> = stored(&pool).await.into_iter().map(|row| row.0).collect();
    assert_eq!(names, ["sda", "vda"]);
    let mut expected = LINUX_LINES.to_vec();
    expected.insert(LINUX_LINES.len() - 1, LSBLK_OLD_LINE);
    assert_eq!(
        heard[..expected.len()],
        expected,
        "the old form before os-release"
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../../../tests/fixtures", scripts("devices"))
)]
async fn met_028_the_second_round_gives_each_drive_its_activity_and_storage_shows_it(pool: PgPool) {
    let mut bench = bench(&pool, true).await;
    bench.collector.round().await;
    // 30 seconds on: nvme1n1 read 30 000 more sectors, wrote 60 000 more, and was busy 3 s.
    let later = String::from_utf8(PROC_DISKSTATS.to_vec()).unwrap().replace(
        "nvme1n1 120000 3000 9000000 40000 80000 9000 6000000 70000 0 50000",
        "nvme1n1 121000 3000 9030000 40000 81000 9000 6060000 70000 0 53000",
    );
    bench.server.answer("cat /proc/diskstats", later.as_bytes());
    bench.clock.advance(30);
    bench.collector.round().await;
    bench.feed.try_recv().unwrap();
    let second = bench.feed.try_recv().unwrap();
    assert_eq!(
        value(&second, MetricKind::DiskReadBps, "nvme1n1"),
        Some(30_000.0 * 512.0 / 30.0)
    );
    assert_eq!(
        value(&second, MetricKind::DiskBusyPercent, "nvme1n1"),
        Some(10.0)
    );
    assert_eq!(value(&second, MetricKind::DiskReadBps, "nvme1n1p1"), None);
    assert_eq!(second.readings.disk_mount.as_deref(), Some("/boot/efi"));
    assert_eq!(second.readings.other_drives, 3, "+3 drives on the card");

    let shown = storage(&pool, device(DOCKER_HOST), bench.clock.now())
        .await
        .unwrap();
    let rows: Vec<(Option<&str>, Option<DriveKind>, bool)> = shown
        .drives
        .iter()
        .map(|row| (row.name.as_deref(), row.kind, row.mounted))
        .collect();
    assert_eq!(
        rows,
        [
            (Some("nvme1n1"), Some(DriveKind::Nvme), true),
            // The capture's df lists /mnt/Backup Drive, which no drive of this lsblk holds.
            (None, None, true),
            (Some("nvme0n1"), Some(DriveKind::Nvme), false),
            (Some("sda"), Some(DriveKind::Hdd), false),
            (Some("sdb"), Some(DriveKind::Hdd), false),
        ]
    );
    assert_eq!(shown.drives[0].busy_percent, Some(10.0));
    assert_eq!(shown.drives[0].mounts[0].percent, Some(25.0));
}
