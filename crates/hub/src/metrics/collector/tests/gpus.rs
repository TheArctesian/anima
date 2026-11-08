//! What each GPU vendor's output says, the AMD files split by card, and the samples of the
//! GPUs a discovery numbered (MET-006, MET-007, MET-009, D-30).

use agent_core::points::Kind;
use agent_core::readings::captures::{
    AMD_GPU_BUSY_PERCENT, AMD_VRAM_TOTAL, AMD_VRAM_USED, INTEL_GPU_TOP_ARRAY, IOREG_ACCELERATOR,
    NVIDIA_SMI_NOT_AVAILABLE, NVIDIA_SMI_TWO,
};
use chrono::{DateTime, Utc};

use crate::{
    metrics::{
        GpuVendor, MAX_LABELS, Sample,
        collector::gpus::{
            Answer, Discovered,
            answers::{OUTPUT_LIMIT, answer},
            discovery::due,
            samples::samples,
            sysfs::cards,
        },
    },
    ssh::CommandOutput,
};

pub(super) fn at() -> DateTime<Utc> {
    "2026-11-07T09:00:00Z".parse().unwrap()
}

fn output(status: Option<u32>, stdout: &[u8]) -> CommandOutput {
    CommandOutput {
        exit_status: status,
        stdout: stdout.to_vec(),
        cut: false,
    }
}

pub(super) fn ok(stdout: &[u8]) -> CommandOutput {
    output(Some(0), stdout)
}

/// `grep -H .` output of one card's three files.
pub(super) fn amd_card(card: u32) -> Vec<u8> {
    let mut out = Vec::new();
    for (file, value) in [
        ("gpu_busy_percent", AMD_GPU_BUSY_PERCENT),
        ("mem_info_vram_used", AMD_VRAM_USED),
        ("mem_info_vram_total", AMD_VRAM_TOTAL),
    ] {
        out.extend(format!("/sys/class/drm/card{card}/device/{file}:").bytes());
        out.extend(value);
    }
    out
}

fn gpus(answer: &Answer) -> &[agent_core::readings::GpuReading] {
    match answer {
        Answer::Gpus(gpus) => gpus,
        Answer::Unknown => panic!("unknown"),
    }
}

/// What a Linux device with two NVIDIA GPUs, AMD card 1 and an Intel GPU answers.
pub(super) fn linux_answers() -> Vec<(GpuVendor, Answer)> {
    vec![
        (
            GpuVendor::Nvidia,
            answer(GpuVendor::Nvidia, &ok(NVIDIA_SMI_TWO)),
        ),
        (GpuVendor::Amd, answer(GpuVendor::Amd, &ok(&amd_card(1)))),
        (
            GpuVendor::Intel,
            answer(GpuVendor::Intel, &ok(INTEL_GPU_TOP_ARRAY)),
        ),
    ]
}

pub(super) fn found(answers: &[(GpuVendor, Answer)]) -> Discovered {
    due(None, None, at()).unwrap().merge(None, answers)
}

fn value(samples: &[Sample], kind: Kind, label: &str) -> Option<f64> {
    samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

#[test]
fn met_006_a_missing_or_refused_tool_is_no_gpu_and_an_unfinished_read_is_unknown() {
    for vendor in [GpuVendor::Nvidia, GpuVendor::Intel, GpuVendor::Apple] {
        assert_eq!(
            answer(vendor, &output(Some(127), b"")),
            Answer::Gpus(Vec::new())
        );
        assert_eq!(
            answer(vendor, &output(Some(1), NVIDIA_SMI_TWO)),
            Answer::Gpus(Vec::new()),
            "a failed tool's output is not read"
        );
    }
    let cut = CommandOutput {
        cut: true,
        ..ok(NVIDIA_SMI_TWO)
    };
    assert_eq!(answer(GpuVendor::Nvidia, &cut), Answer::Unknown);
    assert_eq!(
        answer(GpuVendor::Nvidia, &output(None, NVIDIA_SMI_TWO)),
        Answer::Unknown
    );
    let long = NVIDIA_SMI_TWO.repeat(OUTPUT_LIMIT / NVIDIA_SMI_TWO.len() + 1);
    assert_eq!(answer(GpuVendor::Nvidia, &ok(&long)), Answer::Unknown);
}

#[test]
fn met_006_amd_cards_read_even_when_grep_found_no_file_for_one_glob() {
    let mut printed = amd_card(1);
    printed.extend(b"/sys/class/drm/card2/device/gpu_busy_percent:5\n");
    let read = answer(GpuVendor::Amd, &output(Some(2), &printed));
    let read = gpus(&read);
    assert_eq!(read.len(), 2);
    assert_eq!((read[0].index, read[0].busy_percent), (1, Some(37.0)));
    let vram = read[0].vram.unwrap();
    assert_eq!(
        (vram.used_bytes, vram.total_bytes),
        (2_147_483_648, 17_163_091_968)
    );
    assert_eq!(
        (read[1].index, read[1].busy_percent, read[1].vram),
        (2, Some(5.0), None)
    );
    assert_eq!(read[0].name, "AMD GPU");
    assert_eq!(
        answer(GpuVendor::Amd, &output(Some(2), b"")),
        Answer::Gpus(Vec::new())
    );
}

#[test]
fn met_006_hostile_sysfs_lines_are_skipped_and_cards_are_capped() {
    let mut printed = Vec::new();
    printed.extend(b"/sys/class/drm/card0-DP-1/device/gpu_busy_percent:9\n");
    printed.extend(b"/sys/class/drm/card/device/gpu_busy_percent:9\n");
    printed.extend(b"/etc/passwd:root:x:0:0\n");
    printed.extend(b"/sys/class/drm/card3/device/power_dpm_force_performance_level:9\n");
    printed.extend(b"/sys/class/drm/card3/device/gpu_busy_percent:\xff\xfe\n");
    printed.extend(b"/sys/class/drm/card99999999999/device/gpu_busy_percent:9\n");
    printed.extend(b"/sys/class/drm/card4/device/gpu_busy_percent:12\n");
    printed.extend(b"/sys/class/drm/card4/device/gpu_busy_percent:99\n");
    printed.extend(b"/sys/class/drm/card5/device/gpu_busy_percent:7");
    let read = cards(&printed);
    let seen: Vec<_> = read
        .iter()
        .map(|card| (card.index, card.busy_percent))
        .collect();
    assert_eq!(
        seen,
        [(3, None), (4, Some(12.0))],
        "only card<number>/device/<one of three files>, the first value, complete lines"
    );

    let many: Vec<u8> = (0..1000)
        .flat_map(|card| {
            format!("/sys/class/drm/card{card}/device/gpu_busy_percent:1\n").into_bytes()
        })
        .collect();
    assert_eq!(cards(&many).len(), MAX_LABELS);
}

#[test]
fn met_009_each_gpu_has_samples_labelled_by_its_index() {
    let answers = linux_answers();
    let known = found(&answers);
    let taken = samples(&known, &answers);
    assert_eq!(value(&taken, Kind::GpuPercent, "0"), Some(97.0));
    assert_eq!(
        value(&taken, Kind::VramBytes, "0"),
        Some(20_480.0 * 1_048_576.0)
    );
    let share = value(&taken, Kind::VramPercent, "1").unwrap();
    assert!((share - 1024.0 / 15_360.0 * 100.0).abs() < 1e-9, "{share}");
    assert_eq!(value(&taken, Kind::GpuTemperatureC, "1"), Some(45.0));
    assert_eq!(value(&taken, Kind::GpuPowerW, "0"), Some(412.33));
    assert_eq!(
        value(&taken, Kind::GpuPercent, "2"),
        Some(37.0),
        "AMD card 1"
    );
    assert_eq!(value(&taken, Kind::GpuPercent, "3"), Some(33.25), "Intel");
    assert_eq!(value(&taken, Kind::GpuPowerW, "3"), Some(1.5));
    for sample in &taken {
        assert_eq!(
            sample.gpu_index.map(|index| index.to_string()),
            Some(sample.label.clone())
        );
    }
}

#[test]
fn met_007_no_vram_for_intel_or_apple_and_no_zero_for_a_missing_value() {
    let answers = linux_answers();
    let known = found(&answers);
    let taken = samples(&known, &answers);
    assert_eq!(value(&taken, Kind::VramPercent, "3"), None, "Intel");
    assert_eq!(
        value(&taken, Kind::GpuTemperatureC, "2"),
        None,
        "AMD gives none"
    );

    let laptop = [(
        GpuVendor::Nvidia,
        answer(GpuVendor::Nvidia, &ok(NVIDIA_SMI_NOT_AVAILABLE)),
    )];
    let known = found(&laptop);
    assert!(!known.gpus[1].gpu.has_vram, "memory not supported");
    let taken = samples(&known, &laptop);
    assert_eq!(value(&taken, Kind::GpuPowerW, "0"), None, "[N/A]");
    let second: Vec<Kind> = taken
        .iter()
        .filter(|sample| sample.label == "1")
        .map(|sample| sample.kind)
        .collect();
    assert_eq!(second, [Kind::GpuTemperatureC]);

    let apple = [(
        GpuVendor::Apple,
        answer(GpuVendor::Apple, &ok(IOREG_ACCELERATOR)),
    )];
    let taken = samples(&found(&apple), &apple);
    let kinds: Vec<Kind> = taken.iter().map(|sample| sample.kind).collect();
    assert_eq!(kinds, [Kind::GpuPercent, Kind::GpuPercent]);
}

#[test]
fn met_007_a_gpu_with_no_answer_or_not_yet_found_gives_no_sample() {
    let answers = linux_answers();
    let known = found(&answers[..1]);
    let mut later = answers.clone();
    later[0].1 = Answer::Unknown;
    assert!(samples(&known, &later).is_empty(), "NVIDIA did not answer");
    let taken = samples(&known, &answers);
    assert!(
        taken
            .iter()
            .all(|sample| sample.label == "0" || sample.label == "1"),
        "AMD and Intel wait for the next discovery"
    );
}

#[test]
fn met_009_a_hostile_own_number_still_gives_a_device_number_k8_allows() {
    let hostile = [(
        GpuVendor::Amd,
        answer(GpuVendor::Amd, &ok(&amd_card(u32::MAX))),
    )];
    let known = found(&hostile);
    let numbers: Vec<(i32, u32)> = known
        .gpus
        .iter()
        .map(|found| (found.gpu.index, found.source))
        .collect();
    assert_eq!(numbers, [(0, u32::MAX)], "the lowest free number");
    let taken = samples(&known, &hostile);
    assert_eq!(taken.len(), 3);
    assert!(
        taken
            .iter()
            .all(|sample| sample.gpu_index == Some(0) && sample.label == "0"),
        "{taken:?}"
    );

    let mut beyond = known.clone();
    beyond.gpus[0].gpu.index = 64;
    assert!(
        samples(&beyond, &hostile).is_empty(),
        "a number outside K8 gives no sample"
    );
}
