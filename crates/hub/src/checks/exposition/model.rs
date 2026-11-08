//! What one exposition shows: every check and device with the values of its series, read
//! once and rendered without further reads (INT-001).

use crate::{
    checks::{UptimeWindow, config::CheckType},
    ids::{CheckId, DeviceId, SiteId},
    status::Status,
};

/// The uptime windows of `anima_check_uptime_ratio`, with their fixed `window` label values
/// (D-24). [`CheckSample::uptime`] holds one ratio per entry, in this order.
pub(super) const WINDOWS: [(UptimeWindow, &str); 3] = [
    (UptimeWindow::Day, "1d"),
    (UptimeWindow::Week, "7d"),
    (UptimeWindow::Month, "30d"),
];

/// The device a series names. Only values that rarely change (INT-002).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeviceLabels {
    pub(super) id: DeviceId,
    pub(super) name: String,
    pub(super) site: SiteId,
}

/// One device and the status it shows (CHK-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DeviceSample {
    pub(super) device: DeviceLabels,
    pub(super) status: Status,
}

/// One check and the values of its series.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CheckSample {
    pub(super) id: CheckId,
    pub(super) name: String,
    pub(super) kind: CheckType,
    pub(super) device: DeviceLabels,
    /// The status shown: pause, groups and maintenance applied, as `Check.status`.
    pub(super) status: Status,
    /// The response time of the newest heartbeat; `None` without one, or when it has none.
    pub(super) response_time_ms: Option<u32>,
    /// The ratio for each of [`WINDOWS`]; `None` when that window counted no beats.
    pub(super) uptime: [Option<f64>; 3],
    /// As `Check.certificateDaysLeft`: only TLS and HTTPS checks that have seen a certificate.
    pub(super) certificate_days_left: Option<i32>,
    /// Only domain expiry checks that have read an expiry date.
    pub(super) domain_days_left: Option<i32>,
}

/// Every check and device, in the order the exposition lists them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Snapshot {
    pub(super) checks: Vec<CheckSample>,
    pub(super) devices: Vec<DeviceSample>,
}
