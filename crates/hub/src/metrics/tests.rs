//! Tests of the metrics readers and rates on captured and hostile device output.

use super::{Cpu, Disk, Interface, Load, Memory, Reading, Sample};
use crate::metrics::MetricKind;

mod aggregates;
mod captures;
mod database;
mod df;
mod faces;
mod kinds;
mod linux;
mod macos;
mod privileges;
mod rates;
mod retention;
mod series;

/// Asserts two floats are the same number, apart from rounding of the last digits.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}

/// The value of the sample of `kind` and `label`, if there is one.
fn value(samples: &[Sample], kind: MetricKind, label: &str) -> Option<f64> {
    samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

/// How many samples there are of `kind`.
fn count(samples: &[Sample], kind: MetricKind) -> usize {
    samples.iter().filter(|sample| sample.kind == kind).count()
}

fn interface(name: &str, received_bytes: u64, sent_bytes: u64) -> Interface {
    Interface {
        name: name.to_owned(),
        received_bytes,
        sent_bytes,
    }
}

fn disk(filesystem: &str, mount: &str, total_bytes: u64, used_bytes: u64) -> Disk {
    Disk {
        filesystem: filesystem.to_owned(),
        mount: mount.to_owned(),
        total_bytes,
        used_bytes,
    }
}
