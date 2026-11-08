//! When the hub looks for a device's GPUs, how it numbers what it finds, and which reads a
//! collection runs (MET-006, MET-007), without a device.

use agent_core::readings::captures::IOREG_ACCELERATOR;
use chrono::TimeDelta;

use super::gpus::{at, found, linux_answers, ok};
use crate::{
    devices::OperatingSystem,
    metrics::{
        GpuVendor, MAX_LABELS, Reading,
        collector::gpus::{
            Answer, Plan,
            answers::answer,
            discovery::{EVERY, RESTART_TOLERANCE, boot_at, discovered, due},
        },
    },
};

#[test]
fn met_006_the_first_collection_looks_and_looks_again_after_24_hours() {
    assert!(due(None, None, at()), "nothing known since the hub started");
    let known = found(&linux_answers());
    let soon = at() + EVERY - TimeDelta::seconds(1);
    assert!(!due(Some(&known), None, soon));
    assert!(due(Some(&known), None, at() + EVERY));
    assert!(
        due(Some(&known), None, at() - TimeDelta::seconds(1)),
        "the hub clock went back"
    );
}

#[test]
fn met_006_a_boot_time_later_by_more_than_the_tolerance_is_a_restart() {
    let booted = at() - TimeDelta::days(4);
    let known = discovered(&linux_answers(), at(), Some(booted)).unwrap();
    let now = at() + TimeDelta::minutes(5);
    let jitter = booted + RESTART_TOLERANCE;
    assert!(
        !due(Some(&known), Some(jitter), now),
        "jitter, not a restart"
    );
    assert!(due(Some(&known), Some(jitter + TimeDelta::seconds(1)), now));
    assert!(
        !due(Some(&known), Some(booted - TimeDelta::hours(1)), now),
        "an earlier boot time is no restart"
    );
    assert!(!due(Some(&known), None, now), "no uptime this time");
}

#[test]
fn met_006_the_boot_time_is_the_collection_time_less_the_uptime() {
    let reading = |uptime| Reading {
        uptime_seconds: Some(uptime),
        ..Reading::default()
    };
    assert_eq!(
        boot_at(&reading(3_600.5), at()),
        Some(at() - TimeDelta::milliseconds(3_600_500))
    );
    assert_eq!(boot_at(&Reading::default(), at()), None);
    assert_eq!(
        boot_at(&reading(1e300), at()),
        None,
        "no panic, no boot time"
    );
    assert_eq!(boot_at(&reading(1e15), at()), None, "before the calendar");
}

#[test]
fn met_007_gpus_are_numbered_across_vendors_and_only_nvidia_and_amd_have_vram() {
    let known = found(&linux_answers());
    let seen: Vec<_> = known
        .gpus
        .iter()
        .map(|found| {
            let gpu = &found.gpu;
            (
                gpu.index,
                found.source,
                gpu.vendor,
                gpu.name.as_str(),
                gpu.has_vram,
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            (0, 0, GpuVendor::Nvidia, "NVIDIA GeForce RTX 4090", true),
            (1, 1, GpuVendor::Nvidia, "Tesla T4, Quadro, Mixed", true),
            (2, 1, GpuVendor::Amd, "AMD GPU", true),
            (3, 0, GpuVendor::Intel, "Intel GPU", false),
        ]
    );
    let apple = [(
        GpuVendor::Apple,
        answer(GpuVendor::Apple, &ok(IOREG_ACCELERATOR)),
    )];
    let apple = found(&apple);
    assert_eq!(apple.gpus.len(), 2);
    assert!(apple.gpus.iter().all(|found| !found.gpu.has_vram));
}

#[test]
fn met_006_an_unknown_answer_finds_nothing_and_repeats_and_counts_are_bounded() {
    let mut answers = linux_answers();
    answers[2].1 = Answer::Unknown;
    assert_eq!(discovered(&answers, at(), None), None);

    let repeated: Vec<u8> = (0..1000)
        .flat_map(|index| format!("{}, GPU {index}, 1, 1, 2, 3, 4\n", index % 40).into_bytes())
        .collect();
    let many = [(GpuVendor::Nvidia, answer(GpuVendor::Nvidia, &ok(&repeated)))];
    let known = found(&many);
    assert_eq!(known.gpus.len(), MAX_LABELS);
    let sources: Vec<u32> = known.gpus.iter().map(|found| found.source).collect();
    assert_eq!(
        sources,
        (0..32).collect::<Vec<u32>>(),
        "first of each index"
    );
    assert_eq!(known.gpus[1].gpu.name, "GPU 1");
}

#[test]
fn met_006_a_collection_that_does_not_look_runs_only_the_known_vendors_reads() {
    let answers = linux_answers();
    let known = found(&answers[1..2]);
    let reading = Reading::default();
    let soon = at() + TimeDelta::minutes(1);
    let plan = Plan::new(OperatingSystem::Linux, Some(known.clone()), &reading, soon);
    let vendors: Vec<GpuVendor> = plan
        .commands()
        .iter()
        .map(|command| command.vendor)
        .collect();
    assert_eq!(vendors, [GpuVendor::Amd]);
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
    let mut unknown = answers.clone();
    unknown[0].1 = Answer::Unknown;
    let round = plan.finish(&unknown);
    assert!(!round.discovered_now, "an unfinished look stores nothing");
    assert_eq!(round.known, Some(known), "and keeps what was known");
}
