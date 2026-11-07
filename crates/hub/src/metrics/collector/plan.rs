//! Which devices a round collects (D-2, MET-005). Pure.

use crate::devices::{Device, MetricsSource, OperatingSystem};

/// Whether a round collects `device`: its metrics come over SSH and it runs Linux or macOS.
/// Windows and other systems are skipped without a link (MET-005).
pub(super) fn is_collected(device: &Device) -> bool {
    device.metrics_source == MetricsSource::Ssh
        && matches!(device.os, OperatingSystem::Linux | OperatingSystem::Macos)
}

/// The devices of `listed` that a round collects, in list order.
pub(super) fn due(listed: Vec<Device>) -> Vec<Device> {
    listed.into_iter().filter(is_collected).collect()
}
