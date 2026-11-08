//! The Prometheus text format 0.0.4 of a [`Snapshot`] (INT-001, INT-002). Pure: samples in,
//! text out.

use super::{
    labels::label_value,
    model::{CheckSample, DeviceLabels, DeviceSample, Snapshot, WINDOWS},
};
use crate::status::Status;

/// The value of a status series: 0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown (K7).
pub(super) const fn status_value(status: Status) -> u8 {
    match status {
        Status::Down => 0,
        Status::Up => 1,
        Status::Degraded => 2,
        Status::Maintenance => 3,
        Status::Unknown => 4,
    }
}

/// 1 when the check answers (Up or Degraded), else 0 (K7).
const fn up_value(status: Status) -> u8 {
    match status {
        Status::Up | Status::Degraded => 1,
        Status::Down | Status::Maintenance | Status::Unknown => 0,
    }
}

/// One metric family: its HELP and TYPE lines, then one line per `(labels, value)`.
fn family(
    out: &mut String,
    name: &str,
    help: &str,
    samples: impl Iterator<Item = (String, String)>,
) {
    for line in [
        ["# HELP ", name, " ", help],
        ["# TYPE ", name, " gauge", ""],
    ] {
        out.extend(line);
        out.push('\n');
    }
    for (labels, value) in samples {
        out.extend([name, "{", &labels, "} ", &value, "\n"]);
    }
}

fn device_labels(device: &DeviceLabels) -> String {
    format!(
        "device_id=\"{}\",device_name=\"{}\",site=\"{}\"",
        label_value(&device.id.to_string()),
        label_value(&device.name),
        label_value(&device.site.to_string()),
    )
}

fn check_labels(check: &CheckSample) -> String {
    format!(
        "check_id=\"{}\",check_name=\"{}\",check_type=\"{}\",{}",
        label_value(&check.id.to_string()),
        label_value(&check.name),
        label_value(check.kind.as_sql()),
        device_labels(&check.device),
    )
}

/// The exposition of `snapshot`: every K7 family with its HELP and TYPE lines, a family's
/// samples in snapshot order. A family with no samples keeps its HELP and TYPE lines.
pub(super) fn render(snapshot: &Snapshot) -> String {
    let checks: Vec<(String, &CheckSample)> = snapshot
        .checks
        .iter()
        .map(|check| (check_labels(check), check))
        .collect();
    let mut out = String::new();
    let each = |value: fn(&CheckSample) -> Option<String>| {
        checks
            .iter()
            .filter_map(move |(labels, check)| value(check).map(|v| (labels.clone(), v)))
    };
    family(
        &mut out,
        "anima_check_status",
        "Check status: 0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown or paused.",
        each(|c| Some(status_value(c.status).to_string())),
    );
    family(
        &mut out,
        "anima_check_up",
        "1 when the check is Up or Degraded, else 0.",
        each(|c| Some(up_value(c.status).to_string())),
    );
    family(
        &mut out,
        "anima_check_response_time_seconds",
        "Response time of the check's last heartbeat, in seconds.",
        each(|c| {
            c.response_time_ms
                .map(|ms| (f64::from(ms) / 1000.0).to_string())
        }),
    );
    uptime_family(&mut out, &checks);
    family(
        &mut out,
        "anima_check_certificate_days_left",
        "Whole days until the check's TLS certificate expires.",
        each(|c| c.certificate_days_left.map(|days| days.to_string())),
    );
    family(
        &mut out,
        "anima_check_domain_days_left",
        "Whole days until the checked domain's registration expires.",
        each(|c| c.domain_days_left.map(|days| days.to_string())),
    );
    family(
        &mut out,
        "anima_device_status",
        "Device status: 0 Down, 1 Up, 2 Degraded, 3 Maintenance, 4 Unknown.",
        snapshot.devices.iter().map(device_status_sample),
    );
    out
}

/// `anima_check_uptime_ratio`, one sample per check and window that counted beats (CHK-011).
fn uptime_family(out: &mut String, checks: &[(String, &CheckSample)]) {
    let samples = checks.iter().flat_map(|(labels, check)| {
        WINDOWS
            .iter()
            .zip(check.uptime)
            .filter_map(move |((_, window), ratio)| {
                ratio.map(|ratio| (format!("{labels},window=\"{window}\""), ratio.to_string()))
            })
    });
    family(
        out,
        "anima_check_uptime_ratio",
        "Share of counted beats that were Up or Degraded over the window.",
        samples,
    );
}

fn device_status_sample(sample: &DeviceSample) -> (String, String) {
    (
        device_labels(&sample.device),
        status_value(sample.status).to_string(),
    )
}
