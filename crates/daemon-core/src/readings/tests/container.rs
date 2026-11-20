//! The control group readers of the container of a hosted hub (HMD-071, HMD-072). The figures
//! are those that a hub of the pool showed on 2026-11-20.

use std::time::Duration;

use super::*;
use crate::points::{Kind, samples};
use crate::readings::{Data, Reading, container};

const MEMORY_MAX: &[u8] = b"1610612736\n";
const MEMORY_CURRENT: &[u8] = b"359116800\n";
const MEMORY_STAT: &[u8] =
    b"anon 99278848\nfile 242601984\ninactive_file 159383552\nshmem 38772736\n";
const CPU_STAT: &[u8] = b"usage_usec 412718333\nuser_usec 300000000\nsystem_usec 112718333\n";

#[test]
fn hmd_071_memory_is_the_use_of_the_container_against_its_own_limit() {
    assert_eq!(
        container::memory(MEMORY_MAX, MEMORY_CURRENT, MEMORY_STAT),
        Some(Memory {
            total_bytes: 1_610_612_736,
            used_bytes: 359_116_800 - 159_383_552,
        })
    );
}

#[test]
fn hmd_071_a_container_with_no_memory_limit_gives_no_memory_rather_than_a_total_of_the_server() {
    assert_eq!(
        container::memory(b"max\n", MEMORY_CURRENT, MEMORY_STAT),
        None
    );
    assert_eq!(container::memory(b"", MEMORY_CURRENT, MEMORY_STAT), None);
    assert_eq!(container::memory(MEMORY_MAX, b"", MEMORY_STAT), None);
}

#[test]
fn hmd_071_the_processor_time_is_the_usage_counter_of_the_control_group() {
    assert_eq!(container::cpu_usage_micros(CPU_STAT), Some(412_718_333));
    assert_eq!(container::cpu_usage_micros(b"user_usec 3\n"), None);
}

#[test]
fn hmd_071_the_allowed_processors_are_the_hard_limit_or_the_count_and_never_the_weight() {
    // The pool: no quota (its 25 % is a weight), one processor.
    assert_eq!(container::allowed_processors(b"max 100000\n", 1), 1000);
    assert_eq!(container::allowed_processors(b"50000 100000\n", 4), 500);
    // A quota above the processors that the container may run on counts the processors.
    assert_eq!(container::allowed_processors(b"800000 100000\n", 2), 2000);
    assert_eq!(container::allowed_processors(b"", 0), 1000);
}

#[test]
fn hmd_071_the_processor_share_is_the_time_used_against_the_time_of_the_allowed_processors() {
    let before = Reading {
        cpu: Some(container::cpu_ticks(0, 0, 500)),
        ..Reading::default()
    };
    // Half a processor for 10 seconds, of which 1.25 seconds were used: 25 %.
    let now = Reading {
        cpu: Some(container::cpu_ticks(1_250_000, 10_000_000, 500)),
        ..Reading::default()
    };
    let out = samples(Some(&before), &now, Duration::from_secs(10));
    let share = out
        .iter()
        .find(|sample| sample.kind == Kind::CpuPercent)
        .map(|sample| sample.value);
    assert!(share.is_some_and(|share| (share - 25.0).abs() < 1e-9));
}

#[test]
fn hmd_072_the_data_of_a_hub_is_its_bytes_with_no_share() {
    let reading = Reading {
        data: Some(Data {
            folder: "/var/lib/anima".to_owned(),
            bytes: 163_306_636,
        }),
        ..Reading::default()
    };
    let out = samples(None, &reading, Duration::from_secs(10));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, Kind::DiskBytes);
    assert_eq!(out[0].label, "/var/lib/anima");
    assert!((out[0].value - 163_306_636.0).abs() < 1e-3);
}
