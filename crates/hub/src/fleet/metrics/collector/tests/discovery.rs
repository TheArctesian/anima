//! When the hub looks for a device's GPUs, how it numbers what it finds and keeps the numbers
//! (MET-006, MET-007, MET-009), without a device.

use agent_core::readings::captures::IOREG_ACCELERATOR;
use chrono::{DateTime, TimeDelta, Utc};

use super::gpus::{at, found, linux_answers, ok, output};
use crate::fleet::{
    devices::OperatingSystem,
    metrics::{
        GpuVendor, Reading,
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
fn look(known: Option<&Discovered>, now: DateTime<Utc>) -> Due {
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

/// What the hub knows after the look due at `now` (no boot time) gave `answers`.
fn looked(known: &Discovered, answers: &[(GpuVendor, Answer)], now: DateTime<Utc>) -> Discovered {
    due(Some(known), None, now)
        .unwrap()
        .merge(Some(known), answers)
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
    let merged = looked(&failed, &linux_answers(), soon);
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
    let after = looked(&known, &fewer_amd, later);
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

/// `answers` with NVIDIA's tool exiting `status` with no output.
pub(super) fn failed(answers: &[(GpuVendor, Answer)], status: u32) -> Vec<(GpuVendor, Answer)> {
    let nvidia = answer(GpuVendor::Nvidia, &output(Some(status), b""));
    with(answers.to_vec(), GpuVendor::Nvidia, &nvidia)
}

#[test]
fn met_006_a_known_vendor_whose_tool_fails_keeps_its_gpus_and_is_read_again() {
    let answers = linux_answers();
    let known = found(&answers);
    let later = at() + EVERY;
    let after = looked(&known, &failed(&answers, 9), later);
    assert_eq!(numbers(&after), numbers(&known), "NVIDIA keeps its GPUs");
    assert_eq!(after.pending, [GpuVendor::Nvidia]);
    let next = later + TimeDelta::seconds(30);
    let plan = Plan::new(
        OperatingSystem::Linux,
        Some(after),
        &Reading::default(),
        next,
    );
    assert_eq!(
        reads(&plan),
        [GpuVendor::Nvidia, GpuVendor::Amd, GpuVendor::Intel],
        "NVIDIA is read again the next round"
    );
}

#[test]
fn met_006_a_failing_tool_of_a_vendor_with_no_gpus_or_a_missing_tool_is_no_gpu() {
    let answers = linux_answers();
    let without_nvidia = found(&answers[1..]);
    let later = at() + EVERY;
    let after = looked(&without_nvidia, &failed(&answers, 9), later);
    assert_eq!(
        (after.gpus.len(), after.pending.len()),
        (2, 0),
        "no GPU (D-30)"
    );
    assert_eq!(look(Some(&after), later + TimeDelta::seconds(30)), Due::No);

    let known = found(&answers);
    let missing = looked(&known, &failed(&answers, 127), later);
    assert_eq!(
        numbers(&missing),
        [(2, GpuVendor::Amd, 1), (3, GpuVendor::Intel, 0)],
        "a missing tool drops the known NVIDIA GPUs"
    );
    assert_eq!(missing.pending, []);
}

#[test]
fn met_009_gpus_keep_their_numbers_and_a_new_one_takes_the_lowest_free() {
    let answers = linux_answers();
    let known = found(&answers);
    let later = at() + EVERY;
    let one_nvidia = Answer::Gpus(vec![gpus_of(&answers[0].1)[1].clone()]);
    let dropped = with(answers.clone(), GpuVendor::Nvidia, &one_nvidia);
    let after = looked(&known, &dropped, later);
    assert_eq!(
        numbers(&after),
        [
            (1, GpuVendor::Nvidia, 1),
            (2, GpuVendor::Amd, 1),
            (3, GpuVendor::Intel, 0),
        ],
        "NVIDIA 0 dropped; the others keep their numbers"
    );
    let back = looked(&after, &answers, later + EVERY);
    assert_eq!(
        numbers(&back)[0],
        (0, GpuVendor::Nvidia, 0),
        "the lowest free number"
    );
}

/// The vendors whose reads `plan` runs.
pub(super) fn reads(plan: &Plan) -> Vec<GpuVendor> {
    plan.commands()
        .iter()
        .map(|command| command.vendor)
        .collect()
}

fn gpus_of(answer: &Answer) -> &[agent_core::readings::GpuReading] {
    match answer {
        Answer::Gpus(gpus) => gpus,
        Answer::Failed | Answer::Unknown => &[],
    }
}
