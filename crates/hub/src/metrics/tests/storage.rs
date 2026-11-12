//! MET-021 to MET-026: the Storage rows from the drives and the latest samples, the card's
//! mount and drive count, and the drives store replaced per device.

use std::collections::HashMap;

use agent_core::readings::{Drive, DriveKind as ReadKind, captures::LSBLK_POP_OS, drives};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::database::{DOCKER_HOST, PVE_01, day_at, device, write};
use crate::metrics::{
    DriveKind, MetricKind, Readings, latest, readings, storage,
    storage::{STORAGE_KINDS, storage as rows_of},
    store,
};

fn pop_os() -> Vec<Drive> {
    drives::drives(LSBLK_POP_OS).unwrap()
}

fn latest_of(values: &[(MetricKind, &str, f64)]) -> Vec<latest::Latest> {
    values
        .iter()
        .map(|&(kind, label, value)| latest::Latest {
            device: device(PVE_01),
            kind,
            label: label.to_owned(),
            at: DateTime::<Utc>::UNIX_EPOCH,
            value,
            gpu: None,
        })
        .collect()
}

#[test]
fn met_024_mounted_drives_come_first_then_lone_mounts_then_unmounted_drives() {
    let latest = latest_of(&[
        (MetricKind::DiskPercent, "/", 81.0),
        (MetricKind::DiskBytes, "/", 810.0),
        (MetricKind::DiskPercent, "/mnt/nas", 40.0),
        (MetricKind::DiskReadBps, "nvme1n1", 2048.0),
        (MetricKind::DiskBusyPercent, "nvme1n1", 3.0),
    ]);
    let shown = rows_of(&pop_os(), &latest);
    let rows: Vec<(Option<&str>, Option<DriveKind>, bool)> = shown
        .drives
        .iter()
        .map(|row| (row.name.as_deref(), row.kind, row.mounted))
        .collect();
    assert_eq!(
        rows,
        [
            (Some("nvme1n1"), Some(DriveKind::Nvme), true),
            (None, None, true),
            (Some("nvme0n1"), Some(DriveKind::Nvme), false),
            (Some("sda"), Some(DriveKind::Hdd), false),
            (Some("sdb"), Some(DriveKind::Hdd), false),
        ]
    );
    let system = &shown.drives[0];
    assert_eq!(
        (system.read_bps, system.write_bps, system.busy_percent),
        (Some(2048.0), None, Some(3.0))
    );
    assert_eq!(
        system.mounts[0].size_bytes,
        Some(1000.0),
        "used over the share used"
    );
    assert_eq!(system.mounts[1].path, "/boot/efi");
    assert_eq!(system.mounts[1].percent, None, "no sample yet");
    let nas = &shown.drives[1];
    assert_eq!(nas.mounts[0].path, "/mnt/nas");
    assert_eq!((nas.read_bps, nas.size_bytes), (None, None));
    assert_eq!(shown.drives[3].size_bytes, Some(24_000_277_250_048.0));
    assert_eq!(shown.drives[3].mounts, Vec::new());
}

#[test]
fn met_026_a_device_without_known_drives_lists_its_mounts_with_no_kind_or_activity() {
    let latest = latest_of(&[
        (MetricKind::DiskPercent, "/System/Volumes/Data", 50.0),
        (MetricKind::DiskBytes, "/System/Volumes/Data", 250.0),
        (MetricKind::DiskPercent, "/", 0.0),
        (MetricKind::DiskBytes, "/", 0.0),
    ]);
    let shown = rows_of(&[], &latest);
    let paths: Vec<&str> = shown
        .drives
        .iter()
        .map(|row| row.mounts[0].path.as_str())
        .collect();
    assert_eq!(paths, ["/", "/System/Volumes/Data"]);
    assert!(
        shown
            .drives
            .iter()
            .all(|row| row.kind.is_none() && row.name.is_none())
    );
    assert_eq!(
        shown.drives[0].size_bytes, None,
        "nothing used: the size is unknown"
    );
    assert_eq!(shown.drives[1].size_bytes, Some(500.0));
    assert_eq!(STORAGE_KINDS.len(), 5);
}

fn card(values: &[(MetricKind, &str, f64)], drives: Vec<Drive>) -> Readings {
    let mut readings = latest::readings(latest_of(values));
    let known = HashMap::from([(device(PVE_01), drives)]);
    latest::count_other_drives(&mut readings, &known);
    readings.remove(&device(PVE_01)).unwrap()
}

#[test]
fn met_025_the_card_names_the_fullest_mount_and_counts_the_other_drives() {
    let values = [
        (MetricKind::DiskPercent, "/", 81.0),
        (MetricKind::DiskPercent, "/boot/efi", 6.0),
    ];
    let pop = card(&values, pop_os());
    assert_eq!(
        (pop.disk_percent, pop.disk_mount.as_deref()),
        (Some(81.0), Some("/"))
    );
    assert_eq!(
        pop.other_drives, 3,
        "+3 drives: two unmounted HDDs and an NVMe drive"
    );

    let nas = card(&[(MetricKind::DiskPercent, "/mnt/nas", 90.0)], pop_os());
    assert_eq!(
        nas.other_drives, 4,
        "the fullest mount is on no known drive"
    );
    let mac = card(
        &[(MetricKind::DiskPercent, "/System/Volumes/Data", 50.0)],
        Vec::new(),
    );
    assert_eq!(mac.other_drives, 0);
    let tie = card(
        &[
            (MetricKind::DiskPercent, "/srv", 40.0),
            (MetricKind::DiskPercent, "/home", 40.0),
        ],
        Vec::new(),
    );
    assert_eq!(
        tie.disk_mount.as_deref(),
        Some("/home"),
        "a tie keeps the first bytewise"
    );
}

fn drive(name: &str, mounts: &[&str]) -> Drive {
    Drive {
        name: name.to_owned(),
        kind: ReadKind::Ssd,
        size_bytes: 512,
        mounts: mounts.iter().map(|mount| (*mount).to_owned()).collect(),
    }
}

async fn now(pool: &PgPool) -> DateTime<Utc> {
    day_at(pool, 1, "12:00:00").await
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_022_a_device_s_drives_are_replaced_together_and_another_s_are_left(pool: PgPool) {
    let at = now(&pool).await;
    let (pve, docker) = (device(PVE_01), device(DOCKER_HOST));
    store::replace_drives(&pool, pve, &pop_os(), at)
        .await
        .unwrap();
    store::replace_drives(&pool, docker, &[drive("vda", &["/"])], at)
        .await
        .unwrap();
    store::replace_drives(&pool, pve, &[drive("sdz", &[])], at)
        .await
        .unwrap();
    let stored = store::drives(&pool, &[pve, docker]).await.unwrap();
    assert_eq!(stored[&pve], [drive("sdz", &[])]);
    assert_eq!(stored[&docker], [drive("vda", &["/"])]);
    store::replace_drives(&pool, pve, &[], at).await.unwrap();
    assert!(
        !store::drives(&pool, &[pve])
            .await
            .unwrap()
            .contains_key(&pve)
    );
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../tests/fixtures", scripts("devices"))
)]
async fn met_024_the_storage_face_reads_the_drives_and_the_recent_samples(pool: PgPool) {
    let at = now(&pool).await;
    let pve = device(PVE_01);
    store::replace_drives(&pool, pve, &pop_os(), at)
        .await
        .unwrap();
    write(
        &pool,
        pve,
        at,
        &[
            (MetricKind::DiskPercent, "/", 81.0),
            (MetricKind::DiskBytes, "/", 81.0),
            (MetricKind::DiskWriteBps, "sda", 4096.0),
        ],
    )
    .await;
    let shown = storage(&pool, pve, at).await.unwrap();
    assert_eq!(shown.drives.len(), 4);
    assert_eq!(shown.drives[0].mounts[0].size_bytes, Some(100.0));
    assert_eq!(
        shown.drives[2].write_bps,
        Some(4096.0),
        "an unmounted drive's activity"
    );
    let card = readings(&pool, &[pve], at).await.unwrap();
    assert_eq!(card[&pve].disk_mount.as_deref(), Some("/"));
    assert_eq!(card[&pve].other_drives, 3);
    let later = storage(&pool, pve, at + chrono::TimeDelta::minutes(3))
        .await
        .unwrap();
    assert_eq!(
        later.drives[0].mounts[0].percent, None,
        "older than 2 minutes"
    );
}
