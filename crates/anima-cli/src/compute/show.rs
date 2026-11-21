//! `anima compute policy show [device]`: the device policy of each device, or of one, and
//! whether it takes model work now.

use super::documents::{DevicePolicies, device_policies};
use crate::failure::{Failure, Hint};
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

pub type Row = device_policies::DevicePoliciesDevicePolicies;

#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// Only this device: its ID or its exact name.
    device: Option<String>,
}

impl ShowArgs {
    /// Prints the policies as a table or, with `--json`, as the K1 `DevicePolicy` list.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let mut rows = fetch(&hub).await?;
        if let Some(wanted) = &self.device {
            let one = find(&rows, wanted)?.clone();
            rows = vec![one];
        }
        invocation.output.data(&rows, table)
    }
}

/// The device policy of each device.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(hub: &Hub) -> Result<Vec<Row>, Failure> {
    Ok(hub
        .run::<DevicePolicies>(device_policies::Variables)
        .await?
        .device_policies)
}

/// The row of the device whose ID or exact name is `wanted`.
///
/// # Errors
/// [`Failure::NotFound`] when no device has it; [`Failure::Usage`] when several have the name.
pub fn find<'a>(rows: &'a [Row], wanted: &str) -> Result<&'a Row, Failure> {
    if let Some(row) = rows.iter().find(|row| row.device.id == wanted) {
        return Ok(row);
    }
    let named: Vec<&Row> = rows
        .iter()
        .filter(|row| row.device.name == wanted)
        .collect();
    match named.as_slice() {
        [one] => Ok(one),
        [] => Err(Failure::NotFound {
            message: format!("No device has the ID or name {wanted}."),
            hint: Some(Hint::run("anima device list")),
        }),
        _ => Err(Failure::Usage {
            message: format!("More than one device has the name {wanted}: give its ID."),
            hint: Some(Hint::run("anima device list")),
        }),
    }
}

fn table(rows: &Vec<Row>) -> Table {
    let mut table = Table::new(vec![
        "DEVICE",
        "MEMORY",
        "DISK",
        "QUIET HOURS",
        "MAINS ONLY",
        "IDLE ONLY",
        "MODEL WORK",
    ]);
    for row in rows {
        table.row(vec![
            row.device.name.clone().into(),
            format!("{}%", row.memory_share_percent).into(),
            format!("{}%", row.disk_share_percent).into(),
            quiet(row).into(),
            yes_no(row.mains_only).into(),
            yes_no(row.idle_only).into(),
            verdict(row).into(),
        ]);
    }
    table
}

fn quiet(row: &Row) -> String {
    row.quiet_hours.as_ref().map_or_else(
        || "-".to_owned(),
        |hours| format!("{}-{} {}", hours.start, hours.end, hours.time_zone),
    )
}

fn verdict(row: &Row) -> String {
    match &row.stop_reason {
        None => "yes".to_owned(),
        Some(reason) => format!("no ({})", enum_name(reason).to_lowercase()),
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
