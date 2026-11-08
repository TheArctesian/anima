//! The GPU readers (MET-016, MET-007).

use super::{assert_close, captures::*};
use crate::readings::gpu::{amd, apple, intel, nvidia};
use crate::readings::{GpuVendor, Vram};

const MIB: u64 = 1024 * 1024;

#[test]
fn met_016_nvidia_smi_capture_gives_the_gpu_with_memory_in_bytes() {
    let gpus = nvidia::gpus(NVIDIA_SMI);
    assert_eq!(gpus.len(), 1);
    let gpu = &gpus[0];
    assert_eq!((gpu.index, gpu.name.as_str()), (0, "NVIDIA RTX A5000"));
    assert_eq!(gpu.vendor, GpuVendor::Nvidia);
    assert_close(gpu.busy_percent.unwrap(), 12.0);
    assert_eq!(
        gpu.vram,
        Some(Vram {
            used_bytes: 735 * MIB,
            total_bytes: 24564 * MIB
        })
    );
    assert_close(gpu.temperature_c.unwrap(), 61.0);
    assert_close(gpu.power_w.unwrap(), 30.52);
    assert_eq!(gpu.shared_memory_bytes, None);
}

#[test]
fn met_016_nvidia_smi_gives_one_reading_per_gpu_and_keeps_commas_in_a_name() {
    let gpus = nvidia::gpus(NVIDIA_SMI_TWO);
    assert_eq!(gpus.len(), 2);
    assert_eq!(gpus[1].index, 1);
    assert_eq!(gpus[1].name, "Tesla T4, Quadro, Mixed");
    assert_close(gpus[1].busy_percent.unwrap(), 3.0);
    assert_eq!(gpus[1].vram.unwrap().total_bytes, 15360 * MIB);
    assert_close(gpus[0].power_w.unwrap(), 412.33);
}

#[test]
fn met_007_nvidia_smi_not_available_and_not_supported_are_none_never_zero() {
    let gpus = nvidia::gpus(NVIDIA_SMI_NOT_AVAILABLE);
    assert_eq!(gpus.len(), 2);
    assert_eq!(gpus[0].power_w, None);
    assert_close(gpus[0].busy_percent.unwrap(), 5.0);
    assert_eq!(gpus[1].busy_percent, None);
    assert_eq!(gpus[1].vram, None);
    assert_eq!(gpus[1].power_w, None);
    assert_close(gpus[1].temperature_c.unwrap(), 50.0);
}

#[test]
fn met_016_nvidia_smi_drops_a_last_line_without_its_newline() {
    let cut = &NVIDIA_SMI_TWO[..NVIDIA_SMI_TWO.len() - 1];
    let gpus = nvidia::gpus(cut);
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].index, 0);
}

#[test]
fn met_016_nvidia_smi_skips_garbage_and_keeps_good_lines() {
    let output = b"No devices were found\n\
x, Name, 1, 2, 3, 4, 5\n\
, Empty Index, 1, 2, 3, 4, 5\n\
7, Short, 1, 2\n\
2, Bell \x07 Name, 1, 2, 3, 4, 5\n\
3, Bad \xff Bytes, 1, 2, 3, 4, 5\n\
4, Good, 1, 2, 3, 4, 5\n\
\n";
    let gpus = nvidia::gpus(output);
    assert_eq!(gpus.len(), 1);
    assert_eq!((gpus[0].index, gpus[0].name.as_str()), (4, "Good"));
}

#[test]
fn met_016_nvidia_smi_rejects_negative_nan_and_infinite_numbers_without_losing_the_gpu() {
    let gpus = nvidia::gpus(b"0, Card, -5, 10, 20, NaN, inf\n");
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].busy_percent, None);
    assert_eq!(gpus[0].temperature_c, None);
    assert_eq!(gpus[0].power_w, None);
    assert_eq!(gpus[0].vram.unwrap().used_bytes, 10 * MIB);
}

#[test]
fn met_016_nvidia_smi_memory_that_overflows_bytes_is_none_and_percent_is_not_capped() {
    let output = b"0, Card, 250, 18446744073709551615, 18446744073709551615, 50, 1\n";
    let gpus = nvidia::gpus(output);
    assert_eq!(gpus[0].vram, None);
    assert_close(gpus[0].busy_percent.unwrap(), 250.0);
}

#[test]
fn met_007_nvidia_smi_without_total_memory_has_no_vram() {
    let gpus = nvidia::gpus(b"0, Card, 1, 0, 0, 50, 1\n");
    assert_eq!(gpus[0].vram, None);
}

#[test]
fn met_016_nvidia_smi_empty_output_gives_nothing() {
    assert_eq!(nvidia::gpus(b"").len(), 0);
    assert_eq!(nvidia::gpus(b"\n\n").len(), 0);
}

#[test]
fn met_016_amd_files_give_busy_and_vram_for_one_card() {
    let gpu = amd::card(
        1,
        "Radeon RX 6800",
        amd::CardFiles {
            busy_percent: AMD_GPU_BUSY_PERCENT,
            vram_used: AMD_VRAM_USED,
            vram_total: AMD_VRAM_TOTAL,
        },
    );
    assert_eq!((gpu.index, gpu.vendor), (1, GpuVendor::Amd));
    assert_eq!(gpu.name, "Radeon RX 6800");
    assert_close(gpu.busy_percent.unwrap(), 37.0);
    assert_eq!(
        gpu.vram,
        Some(Vram {
            used_bytes: 2_147_483_648,
            total_bytes: 17_163_091_968
        })
    );
}

#[test]
fn met_007_amd_vram_needs_both_files_and_a_total_above_zero() {
    let files = |used: &'static [u8], total: &'static [u8]| amd::CardFiles {
        busy_percent: b"5\n",
        vram_used: used,
        vram_total: total,
    };
    assert_eq!(amd::card(0, "c", files(b"1\n", b"")).vram, None);
    assert_eq!(amd::card(0, "c", files(b"", b"10\n")).vram, None);
    assert_eq!(amd::card(0, "c", files(b"0\n", b"0\n")).vram, None);
    assert_eq!(amd::card(0, "c", files(b"x\n", b"10\n")).vram, None);
    assert!(amd::card(0, "c", files(b"0\n", b"10\n")).vram.is_some());
}

#[test]
fn met_016_amd_readers_reject_hostile_file_content() {
    assert_eq!(amd::busy_percent(b"-3\n"), None);
    assert_eq!(amd::busy_percent(b"NaN\n"), None);
    assert_eq!(amd::busy_percent(b"\xff\xfe\n"), None);
    assert_eq!(amd::busy_percent(b"50"), None);
    assert_eq!(amd::busy_percent(b""), None);
    assert_eq!(amd::vram_bytes(b"-1\n"), None);
    assert_eq!(amd::vram_bytes(b"99999999999999999999999\n"), None);
    assert_eq!(amd::vram_bytes(b"4096\n"), Some(4096));
}

#[test]
fn met_016_amd_card_with_an_unusable_name_keeps_its_readings_under_a_fixed_name() {
    let files = amd::CardFiles {
        busy_percent: b"5\n",
        vram_used: b"",
        vram_total: b"",
    };
    assert_eq!(amd::card(0, "bad\u{7}name", files).name, "AMD GPU");
    assert_eq!(amd::card(0, "", files).name, "AMD GPU");
}

#[test]
fn met_016_intel_json_array_gives_the_last_sample_with_the_busiest_engine() {
    let gpu = intel::gpu(INTEL_GPU_TOP_ARRAY).unwrap();
    assert_eq!((gpu.index, gpu.vendor), (0, GpuVendor::Intel));
    assert_close(gpu.busy_percent.unwrap(), 33.25);
    assert_close(gpu.power_w.unwrap(), 1.5);
}

#[test]
fn met_007_intel_has_no_vram() {
    let gpu = intel::gpu(INTEL_GPU_TOP_ARRAY).unwrap();
    assert_eq!(gpu.vram, None);
    assert_eq!(gpu.shared_memory_bytes, None);
}

#[test]
fn met_016_intel_old_stream_ignores_a_truncated_last_object() {
    let gpu = intel::gpu(INTEL_GPU_TOP_STREAM_CUT).unwrap();
    assert_close(gpu.busy_percent.unwrap(), 60.0);
}

#[test]
fn met_016_intel_stream_with_braces_inside_strings_still_splits_correctly() {
    let output = br#"{"note": "a } and \" { here", "engines": {"R": {"busy": 8.5}}},"#;
    assert_close(intel::gpu(output).unwrap().busy_percent.unwrap(), 8.5);
}

#[test]
fn met_016_intel_gives_none_without_a_complete_sample_or_a_usable_engine() {
    assert_eq!(intel::gpu(b""), None);
    assert_eq!(intel::gpu(b"[\n{\"engines\": {\"R\": {\"bus"), None);
    assert_eq!(intel::gpu(b"[{\"engines\": {}}]"), None);
    assert_eq!(intel::gpu(b"[{\"power\": {\"GPU\": 1.0}}]"), None);
    assert_eq!(intel::gpu(b"not json {at all"), None);
    assert_eq!(intel::gpu(b"\xff\xfe\x00{{{}}}"), None);
}

#[test]
fn met_016_intel_ignores_negative_busy_and_power() {
    let output =
        br#"[{"engines": {"A": {"busy": -4.0}, "B": {"busy": 2.0}}, "power": {"GPU": -1.0}}]"#;
    let gpu = intel::gpu(output).unwrap();
    assert_close(gpu.busy_percent.unwrap(), 2.0);
    assert_eq!(gpu.power_w, None);
}

#[test]
fn met_016_ioreg_gives_one_reading_per_accelerator_in_order() {
    let gpus = apple::gpus(IOREG_ACCELERATOR);
    assert_eq!(gpus.len(), 2);
    assert_eq!((gpus[0].index, gpus[1].index), (0, 1));
    assert_eq!(gpus[0].name, "Apple M2 Pro");
    assert_eq!(gpus[0].vendor, GpuVendor::Apple);
    assert_close(gpus[0].busy_percent.unwrap(), 17.0);
    assert_eq!(gpus[0].shared_memory_bytes, Some(987_654_321));
    assert_close(gpus[1].busy_percent.unwrap(), 3.0);
    assert_eq!(gpus[1].shared_memory_bytes, Some(1000));
}

#[test]
fn met_007_apple_has_no_vram_and_no_figure_it_was_not_given() {
    let gpus = apple::gpus(IOREG_ACCELERATOR);
    assert!(gpus.iter().all(|gpu| gpu.vram.is_none()));
    let only_model = apple::gpus(b"+-o AGX  <class X>\n  {\n    \"model\" = \"Apple M1\"\n  }\n");
    assert_eq!(only_model.len(), 1);
    assert_eq!(only_model[0].busy_percent, None);
    assert_eq!(only_model[0].shared_memory_bytes, None);
}

#[test]
fn met_016_ioreg_drops_a_last_line_without_its_newline() {
    let output = b"+-o AGX  <class X>\n    \"model\" = \"Apple M1\"\n    \"PerformanceStatistics\" = {\"Device Utilization %\"=9}";
    let gpus = apple::gpus(output);
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].busy_percent, None);
}

#[test]
fn met_016_ioreg_reads_hostile_values_without_failing() {
    let output = b"+-o AGX  <class X>\n\
    \"model\" = \"Bad \x07 Name\"\n\
    \"PerformanceStatistics\" = {\"Device Utilization %\"=-5,\"In use system memory\"=99999999999999999999999}\n\
+-o AGX2  <class X>\n\
    \"PerformanceStatistics\" = {\"Device Utilization %\"=NaN}\n\
+-o AGX3  <class X>\n\
    \"IOClass\" = \"nothing useful\"\n\
    \xff\xfe garbage\n\
+-o AGX4  <class X>\n\
    \"model\" = \"Apple M9\"\n\
    \"PerformanceStatistics\" = {\"Device Utilization %\"=-5,\"In use system memory\"=99999999999999999999999}\n";
    let gpus = apple::gpus(output);
    assert_eq!(gpus.len(), 1);
    // Entries with nothing usable are skipped; the unusable name of the first is dropped.
    assert_eq!((gpus[0].index, gpus[0].name.as_str()), (0, "Apple M9"));
    assert_eq!(gpus[0].busy_percent, None);
    assert_eq!(gpus[0].shared_memory_bytes, None);
}

#[test]
fn met_016_ioreg_empty_output_gives_nothing() {
    assert_eq!(apple::gpus(b"").len(), 0);
}
