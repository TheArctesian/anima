//! Tests of the device-output readers on captured and hostile output.

use super::{Cpu, Disk, Interface, Load, Memory, captures};

mod df;
mod linux;
mod macos;

/// Asserts two floats are the same number, apart from rounding of the last digits.
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
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
