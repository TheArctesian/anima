//! Which devices a round collects, and which it first asks for their operating system (D-2,
//! MET-005, DEV-025). Pure.

use crate::devices::{Device, MetricsSource, OperatingSystem};

/// Whether a round collects `device`: its metrics come over SSH and it runs Linux or macOS.
/// Windows is skipped without a link (MET-005); a device at Other goes through detection
/// first (DEV-025) and is collected once its operating system is stored.
pub(super) fn is_collected(device: &Device) -> bool {
    device.metrics_source == MetricsSource::Ssh
        && matches!(device.os, OperatingSystem::Linux | OperatingSystem::Macos)
}

/// The devices of `listed` that a round collects, in list order.
pub(super) fn due(listed: Vec<Device>) -> Vec<Device> {
    listed.into_iter().filter(is_collected).collect()
}

/// Whether a round asks `device` for its operating system: its metrics come over SSH and its
/// operating system is Other (DEV-025).
fn awaits_os(device: &Device) -> bool {
    device.metrics_source == MetricsSource::Ssh && device.os == OperatingSystem::Other
}

/// The devices of `listed` whose operating system a round detects, in list order.
pub(super) fn awaiting_os(listed: &[Device]) -> Vec<Device> {
    listed
        .iter()
        .filter(|device| awaits_os(device))
        .cloned()
        .collect()
}
