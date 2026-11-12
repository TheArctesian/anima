//! Tests of the rates, the label rules, the window and the backlog on captured and hostile
//! input.

use chrono::{DateTime, Utc};

use super::{Kind, Sample};
use crate::readings::{Cpu, Disk, Interface, Load, Memory, Reading};

mod backlog;
mod key;
mod labels;
mod rates;
mod sample;
mod window;

/// Asserts two floats are the same number, apart from rounding of the last digits.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}

/// The value of the sample of `kind` and `label`, if there is one.
fn value(samples: &[Sample], kind: Kind, label: &str) -> Option<f64> {
    samples
        .iter()
        .find(|sample| sample.kind == kind && sample.label == label)
        .map(|sample| sample.value)
}

/// How many samples there are of `kind`.
fn count(samples: &[Sample], kind: Kind) -> usize {
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
        available_bytes: total_bytes.saturating_sub(used_bytes),
    }
}

/// The UTC time written as RFC 3339 `text`.
fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}
