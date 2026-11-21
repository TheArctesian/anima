//! The batteries of a device with their charge (MET-032). K1 gives each battery's last state
//! with the device and keeps its charge as the series `BATTERY_PERCENT`, labelled with the
//! battery's name; `anima device get` and `anima_device_get` give both.

use serde_json::{Value, json};

use crate::devices::documents::{DeviceBatteryCharge, device_battery_charge, device_get};
use crate::hub::{Hub, HubError};
use crate::output::Table;
use crate::output::text::enum_name;

type Series = device_battery_charge::DeviceBatteryChargeDeviceMetrics;

/// The latest charge, 0 to 100, of each battery that `names` lists, in that order; `None` for
/// a battery with no reading in the last hour. Sends no request when `names` is empty.
///
/// # Errors
/// The hub's refusal.
pub async fn charges(hub: &Hub, id: &str, names: &[&str]) -> Result<Vec<Option<f64>>, HubError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let variables = device_battery_charge::Variables { id: id.to_owned() };
    let series = hub
        .run::<DeviceBatteryCharge>(variables)
        .await?
        .device
        .map_or_else(Vec::new, |device| device.metrics);
    Ok(names.iter().map(|name| latest(&series, name)).collect())
}

fn latest(series: &[Series], name: &str) -> Option<f64> {
    series
        .iter()
        .find(|series| {
            matches!(
                series.kind,
                device_battery_charge::MetricKind::BATTERY_PERCENT
            ) && series.label == name
        })
        .and_then(|series| series.points.last())
        .map(|point| point.value)
}

/// `device`, a serialised K1 `Device`, with `chargePercent` added to each of its `batteries`
/// from `charges`, in the same order.
#[must_use]
pub fn with_charges(mut device: Value, charges: &[Option<f64>]) -> Value {
    if let Some(batteries) = device.get_mut("batteries").and_then(Value::as_array_mut) {
        for (battery, charge) in batteries.iter_mut().zip(charges) {
            battery["chargePercent"] = json!(charge);
        }
    }
    device
}

/// One row for each battery: its charge, its state, the time left and its health.
pub fn rows(
    table: &mut Table,
    batteries: &[device_get::DeviceGetDeviceBatteries],
    charges: &[Option<f64>],
) {
    for (index, battery) in batteries.iter().enumerate() {
        let charge = charges.get(index).copied().flatten();
        table.pair(
            &format!("Battery {}", battery.name),
            describe(battery, charge),
        );
    }
}

fn describe(battery: &device_get::DeviceGetDeviceBatteries, charge: Option<f64>) -> String {
    let mut parts = vec![
        charge.map_or_else(|| "-".to_owned(), |percent| format!("{percent:.0} %")),
        state(&enum_name(&battery.state)).to_owned(),
    ];
    if let Some(seconds) = battery.seconds_left {
        parts.push(format!("{} left", duration(seconds)));
    }
    if let Some(health) = battery.health_percent {
        parts.push(format!("health {health} %"));
    }
    parts.join(", ")
}

fn state(name: &str) -> &'static str {
    match name {
        "CHARGING" => "charging",
        "DISCHARGING" => "discharging",
        "FULL" => "full",
        "NOT_CHARGING" => "on mains, not charging",
        _ => "state unknown",
    }
}

fn duration(seconds: i64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}
