//! When the hub looks for a device's GPUs, how it numbers what it finds and keeps the numbers,
//! and which reads a collection runs (MET-006, MET-007, MET-009), without a device.

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
            discovery::{Discovered, EVERY, RESTART_TOLERANCE, SETTLE, boot_at, due},
        },
    },
};

/// Which look is due.
#[derive(Debug, PartialEq)]
enum Due {
    No,
    All,
    Only(Vec<GpuVendor>),
}

/// The look due for `known` at `now` with no boot time.
fn look(known: Option<&Discovered>, now: chrono::DateTime<chrono::Utc>) -> Due {
    match due(known, None, now) {
        None => Due::No,
        Some(look) => look.only.map_or(Due::All, Due::Only),
    }
}

/// `answers` with the one of `vendor` replaced by `answer`.
fn with(
    mut answers: Vec<(GpuVendor, Answer)>,
    vendor: GpuVendor,
    answer: &Answer,
) -> Vec<(GpuVendor, Answer)> {
    for (each, slot) in &mut answers {
        if *each == vendor {
            *slot = answer.clone();
        }
    }
    answers
}

/// Device number, vendor and own number of each known GPU.
fn numbers(known: &Discovered) -> Vec<(i32, GpuVendor, u32)> {
    known
        .gpus
        .iter()
        .map(|found| (found.gpu.index, found.gpu.vendor, found.source))
        .collect()
}

#[test]
fn met_006_the_first_collection_looks_and_looks_again_after_24_hours() {
    assert_eq!(
        look(None, at()),
        Due::All,
        "nothing known since the hub started"
    );
    let known = found(&linux_answers());
    let soon = at() + EVERY - TimeDelta::seconds(1);
    assert_eq!(look(Some(&known), soon), Due::No);
    assert_eq!(look(Some(&known), at() + EVERY), Due::All);
    assert_eq!(
        look(Some(&known), at() - TimeDelta::seconds(1)),
        Due::All,
        "the hub clock went back"
    );
    let failed = Discovered {
        stale: true,
        ..known.clone()
    };
    assert_eq!(look(Some(&failed), soon), Due::All, "a collection failed");
    let merged = due(Some(&failed), None, soon)
        .unwrap()
        .merge(Some(&failed), &linux_answers());
    assert_eq!(numbers(&merged), numbers(&known), "the numbers are kept");
    assert!(!merged.stale);
}

#[test]
fn met_006_a_boot_time_later_by_more_than_the_tolerance_is_a_restart() {
    let booted = at() - TimeDelta::days(4);
    let known = due(None, Some(booted), at())
        .unwrap()
        .merge(None, &linux_answers());
    let now = at() + TimeDelta::minutes(5);
    let jitter = booted + RESTART_TOLERANCE;
    assert!(
        due(Some(&known), Some(jitter), now).is_none(),
        "jitter, not a restart"
    );
    assert!(due(Some(&known), Some(jitter + TimeDelta::seconds(1)), now).is_some());
    assert!(
        due(Some(&known), Some(booted - TimeDelta::hours(1)), now).is_none(),
        "an earlier boot time is no restart"
    );
    assert!(
        due(Some(&known), None, now).is_none(),
        "no uptime this time"
    );
}

#[test]
fn met_006_a_look_soon_after_boot_is_made_again_each_round_until_five_minutes() {
    let answers = linux_answers();
    let booted = at() - TimeDelta::seconds(20);
    let not_ready = with(
        answers.clone(),
        GpuVendor::Nvidia,
        &Answer::Gpus(Vec::new()),
    );
    let early = due(None, Some(booted), at())
        .unwrap()
        .merge(None, &not_ready);
    assert_eq!(early.gpus.len(), 2, "the NVIDIA driver was not ready");
    let next = at() + TimeDelta::seconds(30);
    let again = due(Some(&early), Some(booted), next).unwrap();
    assert_eq!(again.only, None, "every vendor again");
    let ready = again.merge(Some(&early), &answers);
    assert_eq!(ready.gpus.len(), 4);
    let settled_at = booted + SETTLE;
    let settled = due(Some(&ready), Some(booted), settled_at)
        .unwrap()
        .merge(Some(&ready), &answers);
    assert!(
        due(
            Some(&settled),
            Some(booted),
            settled_at + TimeDelta::seconds(30)
        )
        .is_none()
    );
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
fn met_006_an_unknown_vendor_keeps_its_gpus_and_alone_is_looked_at_again() {
    let answers = linux_answers();
    let known = found(&answers);
    let later = at() + EVERY;
    let hung = with(answers.clone(), GpuVendor::Intel, &Answer::Unknown);
    let fewer_amd = with(hung, GpuVendor::Amd, &Answer::Gpus(Vec::new()));
    let after = due(Some(&known), None, later)
        .unwrap()
        .merge(Some(&known), &fewer_amd);
    assert_eq!(
        numbers(&after),
        [
            (0, GpuVendor::Nvidia, 0),
            (1, GpuVendor::Nvidia, 1),
            (3, GpuVendor::Intel, 0),
        ],
        "AMD answered with none; Intel did not answer, so it keeps its GPU"
    );
    assert_eq!(after.pending, [GpuVendor::Intel]);
    assert_eq!(after.found_at, later);
    let next = later + TimeDelta::seconds(30);
    assert_eq!(look(Some(&after), next), Due::Only(vec![GpuVendor::Intel]));
    let pending = due(Some(&after), None, next).unwrap();
    let done = pending.merge(Some(&after), &answers);
    assert_eq!(done.pending, []);
    assert_eq!(
        done.found_at, later,
        "a look at one vendor keeps the 24 hours"
    );
    assert_eq!(numbers(&done).len(), 3, "only Intel was looked at");
}

#[test]
fn met_009_gpus_keep_their_numbers_and_a_new_one_takes_the_lowest_free() {
    let answers = linux_answers();
    let known = found(&answers);
    let later = at() + EVERY;
    let one_nvidia = Answer::Gpus(vec![gpus_of(&answers[0].1)[1].clone()]);
    let dropped = with(answers.clone(), GpuVendor::Nvidia, &one_nvidia);
    let after = due(Some(&known), None, later)
        .unwrap()
        .merge(Some(&known), &dropped);
    assert_eq!(
        numbers(&after),
        [
            (1, GpuVendor::Nvidia, 1),
            (2, GpuVendor::Amd, 1),
            (3, GpuVendor::Intel, 0),
        ],
        "NVIDIA 0 dropped; the others keep their numbers"
    );
    let back = due(Some(&after), None, later + EVERY)
        .unwrap()
        .merge(Some(&after), &answers);
    assert_eq!(
        numbers(&back)[0],
        (0, GpuVendor::Nvidia, 0),
        "the lowest free number"
    );
}

#[test]
fn met_006_repeated_numbers_keep_the_first_and_counts_are_bounded() {
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

fn gpus_of(answer: &Answer) -> &[agent_core::readings::GpuReading] {
    match answer {
        Answer::Gpus(gpus) => gpus,
        Answer::Unknown => &[],
    }
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
    let round = plan.finish(&answers);
    assert!(round.discovered_now, "a look is stored");
    let known = round.known.unwrap();
    assert_eq!(
        numbers(&known)[0],
        (0, GpuVendor::Amd, 1),
        "AMD keeps its number"
    );
    assert_eq!(known.gpus.len(), 4);
}
