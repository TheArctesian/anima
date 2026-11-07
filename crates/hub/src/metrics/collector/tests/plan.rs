//! Which devices a round collects (D-2, MET-005).

use sqlx::PgPool;

use crate::{
    devices::{self, MetricsSource, OperatingSystem},
    metrics::collector::plan::{due, is_collected},
};

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn met_005_only_ssh_metrics_devices_on_linux_or_macos_are_collected(pool: PgPool) {
    let template = devices::list(&pool).await.unwrap().remove(0);
    let systems = [
        (OperatingSystem::Linux, true),
        (OperatingSystem::Macos, true),
        (OperatingSystem::Windows, false),
        (OperatingSystem::Other, false),
    ];
    for (os, collected) in systems {
        let mut device = template.clone();
        device.os = os;
        device.metrics_source = MetricsSource::Ssh;
        assert_eq!(is_collected(&device), collected, "{os:?} over SSH");
        device.metrics_source = MetricsSource::None;
        assert!(!is_collected(&device), "{os:?} without a metrics source");
    }
}

#[sqlx::test(
    migrations = "./migrations",
    fixtures(path = "../../../../tests/fixtures", scripts("devices"))
)]
async fn d2_due_keeps_the_collected_devices_in_list_order(pool: PgPool) {
    let mut listed = devices::list(&pool).await.unwrap();
    assert!(
        due(listed.clone()).is_empty(),
        "no fixture device has SSH metrics"
    );
    for device in &mut listed {
        device.metrics_source = MetricsSource::Ssh;
    }
    let expected: Vec<_> = listed
        .iter()
        .filter(|device| device.os != OperatingSystem::Windows)
        .map(|device| device.id)
        .collect();
    let chosen: Vec<_> = due(listed).iter().map(|device| device.id).collect();
    assert_eq!(chosen, expected);
    assert_eq!(chosen.len(), 12, "13 fixture devices, one on Windows");
}
