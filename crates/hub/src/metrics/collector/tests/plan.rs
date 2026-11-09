//! What a round plans: the devices it collects (D-2, MET-005) and the GPU reads each
//! collection runs (MET-006).

use chrono::TimeDelta;
use sqlx::PgPool;

use super::{
    discovery::reads,
    gpus::{at, found, linux_answers},
};
use crate::{
    devices::{self, MetricsSource, OperatingSystem},
    metrics::{
        GpuVendor, Reading,
        collector::{
            gpus::{Answer, Plan, discovery::EVERY},
            plan::{due, is_collected},
        },
    },
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

#[test]
fn met_006_a_collection_that_does_not_look_runs_only_the_known_vendors_reads() {
    let answers = linux_answers();
    let known = found(&answers[1..2]);
    let reading = Reading::default();
    let soon = at() + TimeDelta::minutes(1);
    let plan = Plan::new(OperatingSystem::Linux, Some(known.clone()), &reading, soon);
    assert_eq!(reads(&plan), [GpuVendor::Amd]);
    let round = plan.finish(&answers[1..2]);
    assert!(!round.discovered_now);
    assert_eq!(round.known, Some(known.clone()));
    assert_eq!(round.samples.len(), 3, "busy, VRAM share and bytes");

    let none = found(&[(GpuVendor::Nvidia, Answer::Gpus(Vec::new()))]);
    let plan = Plan::new(OperatingSystem::Linux, Some(none), &reading, soon);
    assert!(plan.commands().is_empty(), "no GPU, no GPU read");

    let plan = Plan::new(
        OperatingSystem::Linux,
        Some(known.clone()),
        &reading,
        at() + EVERY,
    );
    assert_eq!(plan.commands().len(), 3, "a look runs every read");
    let round = plan.finish(&answers);
    assert!(round.discovered_now, "a look is stored");
    let known = round.known.unwrap();
    let amd = &known.gpus[0];
    assert_eq!(
        (amd.gpu.index, amd.gpu.vendor, amd.source),
        (0, GpuVendor::Amd, 1),
        "AMD keeps its number"
    );
    assert_eq!(known.gpus.len(), 4);
}
