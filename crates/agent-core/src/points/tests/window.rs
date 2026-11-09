//! One point per 10-second window (MET-014, K8 Section 7).

use super::*;
use crate::points::{MAX_SERIES, Point, SeriesKey, Summary, Window};

fn plain(kind: Kind) -> SeriesKey {
    SeriesKey::new(kind, None, None).unwrap()
}

fn gpu(kind: Kind, index: u8) -> SeriesKey {
    SeriesKey::new(kind, None, Some(index)).unwrap()
}

fn summary(point: &Point, key: &SeriesKey) -> Option<Summary> {
    point.values().get(key).copied()
}

#[test]
fn met_014_a_point_holds_the_mean_and_for_cpu_and_gpu_use_the_max() {
    let mut window = Window::new();
    let cpu = plain(Kind::CpuPercent);
    let memory = plain(Kind::MemoryBytes);
    let busy = gpu(Kind::GpuPercent, 1);
    for (second, value) in [(2, 10.0), (4, 20.0), (6, 30.0), (8, 40.0), (10, 50.0)] {
        let time = at(&format!("2026-11-08T10:00:{second:02}Z"));
        let values = [
            (cpu.clone(), value),
            (memory.clone(), value * 100.0),
            (busy.clone(), 100.0 - value),
        ];
        assert_eq!(window.add(time, values), None, "the window is still open");
    }
    let point = window.add(at("2026-11-08T10:00:12Z"), []).unwrap();
    assert_eq!(point.at(), at("2026-11-08T10:00:10Z"));
    let cpu = summary(&point, &cpu).unwrap();
    assert_close(cpu.mean, 30.0);
    assert_eq!(cpu.max, Some(50.0));
    let memory = summary(&point, &memory).unwrap();
    assert_close(memory.mean, 3000.0);
    assert_eq!(memory.max, None);
    let busy = summary(&point, &busy).unwrap();
    assert_close(busy.mean, 70.0);
    assert_eq!(busy.max, Some(90.0));
}

#[test]
fn met_014_a_series_with_no_sample_in_a_window_is_absent_from_its_point() {
    let mut window = Window::new();
    let cpu = plain(Kind::CpuPercent);
    let load = plain(Kind::Load1);
    window.add(
        at("2026-11-08T10:00:02Z"),
        [(cpu.clone(), 1.0), (load.clone(), 2.0)],
    );
    let first = window
        .add(at("2026-11-08T10:00:12Z"), [(cpu.clone(), 3.0)])
        .unwrap();
    let second = window.add(at("2026-11-08T10:00:22Z"), []).unwrap();
    assert!(summary(&first, &load).is_some());
    assert_eq!(summary(&second, &load), None);
    assert_eq!(second.values().len(), 1);
}

#[test]
fn met_014_a_window_in_which_nothing_was_kept_makes_no_point() {
    let mut window = Window::new();
    let cpu = plain(Kind::CpuPercent);
    window.add(
        at("2026-11-08T10:00:02Z"),
        [(cpu.clone(), f64::NAN), (cpu.clone(), -1.0)],
    );
    assert_eq!(window.add(at("2026-11-08T10:00:12Z"), [(cpu, 1.0)]), None);
}

#[test]
fn cr_17_a_gpu_reading_past_its_ceiling_never_reaches_the_mean() {
    let mut window = Window::new();
    let heat = gpu(Kind::GpuTemperatureC, 0);
    let power = gpu(Kind::GpuPowerW, 0);
    window.add(
        at("2026-11-08T10:00:02Z"),
        [(heat.clone(), 1000.0), (power.clone(), 10_000.0)],
    );
    window.add(
        at("2026-11-08T10:00:04Z"),
        [(heat.clone(), 1000.5), (power.clone(), f64::MAX)],
    );
    let point = window.add(at("2026-11-08T10:00:12Z"), []).unwrap();
    assert_eq!(
        summary(&point, &heat).unwrap().mean.to_bits(),
        1000.0_f64.to_bits()
    );
    assert_eq!(
        summary(&point, &power).unwrap().mean.to_bits(),
        10_000.0_f64.to_bits()
    );
}

#[test]
fn k8_points_end_on_multiples_of_10_seconds_of_utc() {
    let cpu = plain(Kind::CpuPercent);
    for (taken, end) in [
        ("2026-11-08T10:00:03.5Z", "2026-11-08T10:00:10Z"),
        ("2026-11-08T10:00:10Z", "2026-11-08T10:00:10Z"),
        ("2026-11-08T10:00:10.001Z", "2026-11-08T10:00:20Z"),
        ("2026-11-08T10:00:00.5+01:00", "2026-11-08T09:00:10Z"),
        ("2026-11-08T23:59:59Z", "2026-11-09T00:00:00Z"),
        ("1969-12-31T23:59:55Z", "1970-01-01T00:00:00Z"),
    ] {
        let mut window = Window::new();
        window.add(at(taken), [(cpu.clone(), 1.0)]);
        let point = window.add(at("2100-01-01T00:00:00Z"), []).unwrap();
        assert_eq!(point.at(), at(end), "{taken}");
    }
}

#[test]
fn met_014_a_late_sampling_closes_the_open_window_and_starts_its_own() {
    let mut window = Window::new();
    let cpu = plain(Kind::CpuPercent);
    window.add(at("2026-11-08T10:00:08Z"), [(cpu.clone(), 10.0)]);
    let first = window
        .add(at("2026-11-08T10:00:47Z"), [(cpu.clone(), 90.0)])
        .unwrap();
    assert_eq!(first.at(), at("2026-11-08T10:00:10Z"));
    assert_close(summary(&first, &cpu).unwrap().mean, 10.0);
    let back = window
        .add(at("2026-11-08T10:00:21Z"), [(cpu.clone(), 5.0)])
        .unwrap();
    assert_eq!(back.at(), at("2026-11-08T10:00:50Z"), "the clock went back");
    let last = window.add(at("2026-11-08T10:00:31Z"), []).unwrap();
    assert_eq!(last.at(), at("2026-11-08T10:00:30Z"));
}

#[test]
fn met_014_a_mean_stays_within_its_samples_and_percents_within_100() {
    let mut window = Window::new();
    let cpu = plain(Kind::CpuPercent);
    let busy = gpu(Kind::GpuPercent, 0);
    let disk = SeriesKey::new(Kind::DiskBytes, Some("/".to_owned()), None).unwrap();
    for (second, share) in [(2, 0.1), (4, 0.2), (6, 0.3), (8, 100.0 / 3.0)] {
        let time = at(&format!("2026-11-08T10:00:{second:02}Z"));
        let values = [
            (cpu.clone(), share),
            (busy.clone(), 250.0),
            (disk.clone(), f64::MAX),
        ];
        window.add(time, values);
    }
    let point = window.add(at("2026-11-08T10:00:12Z"), []).unwrap();
    let cpu = summary(&point, &cpu).unwrap();
    assert!(cpu.mean >= 0.1 && cpu.mean <= cpu.max.unwrap(), "{cpu:?}");
    let busy = summary(&point, &busy).unwrap();
    assert_eq!((busy.mean, busy.max), (100.0, Some(100.0)));
    assert_eq!(
        summary(&point, &disk).unwrap().mean,
        f64::MAX,
        "no overflow"
    );
}

#[test]
fn k8_a_window_keeps_at_most_256_series() {
    let mut window = Window::new();
    let disks = (0..=MAX_SERIES).map(|n| {
        let key = SeriesKey::new(Kind::DiskBytes, Some(format!("/m{n:03}")), None).unwrap();
        (key, 1.0)
    });
    window.add(at("2026-11-08T10:00:02Z"), disks);
    let point = window.add(at("2026-11-08T10:00:12Z"), []).unwrap();
    assert_eq!(point.values().len(), MAX_SERIES);
}
