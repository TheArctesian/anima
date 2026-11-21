//! `anima compute policy set <device>`: changes the named values of the device policy of a
//! device and keeps the others. It sets no switch: the one switch "takes work" stays at the
//! machine (USE-004, OD-C8).

use super::documents::{SetDevicePolicy, set_device_policy};
use super::show::{Row, fetch, find};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct SetArgs {
    /// The device's ID or its exact name.
    device: String,
    /// The largest part of the usable memory that models may use, 1 to 100 percent.
    #[arg(long)]
    memory_share: Option<i64>,
    /// The largest part of the disk that model files may use, 1 to 100 percent.
    #[arg(long)]
    disk_share: Option<i64>,
    /// Quiet hours as HH:MM-HH:MM, for example 22:00-07:00; needs --time-zone.
    #[arg(long, requires = "time_zone", conflicts_with = "no_quiet_hours")]
    quiet_hours: Option<String>,
    /// The IANA time zone of the quiet hours, for example Europe/London.
    #[arg(long)]
    time_zone: Option<String>,
    /// Removes the quiet hours.
    #[arg(long)]
    no_quiet_hours: bool,
    /// Model work only on mains power: true or false.
    #[arg(long)]
    mains_only: Option<bool>,
    /// No new model work while the operator is active at the device: true or false.
    #[arg(long)]
    idle_only: Option<bool>,
}

impl SetArgs {
    /// Prints the new policy as a record or, with `--json`, as the K1 `DevicePolicy`.
    ///
    /// # Errors
    /// [`Failure::Usage`] for quiet hours that are not HH:MM-HH:MM; [`Failure::NotFound`] for an
    /// unknown device; the hub's refusal (`VALIDATION` names the value out of range).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let rows = fetch(&hub).await?;
        let current = find(&rows, &self.device)?;
        let variables = set_device_policy::Variables {
            device_id: current.device.id.clone(),
            input: self.input(current)?,
        };
        let answer = hub.run::<SetDevicePolicy>(variables).await?;
        invocation.output.data(&answer.set_device_policy, table)
    }

    fn input(&self, current: &Row) -> Result<set_device_policy::DevicePolicyInput, Failure> {
        Ok(set_device_policy::DevicePolicyInput {
            memory_share_percent: self.memory_share.unwrap_or(current.memory_share_percent),
            disk_share_percent: self.disk_share.unwrap_or(current.disk_share_percent),
            quiet_hours: self.quiet(current)?,
            mains_only: self.mains_only.unwrap_or(current.mains_only),
            idle_only: self.idle_only.unwrap_or(current.idle_only),
        })
    }

    fn quiet(&self, current: &Row) -> Result<Option<set_device_policy::QuietHoursInput>, Failure> {
        if self.no_quiet_hours {
            return Ok(None);
        }
        let Some(text) = &self.quiet_hours else {
            return Ok(current.quiet_hours.as_ref().map(|hours| {
                set_device_policy::QuietHoursInput {
                    start: hours.start.clone(),
                    end: hours.end.clone(),
                    time_zone: hours.time_zone.clone(),
                }
            }));
        };
        let (start, end) = text.split_once('-').ok_or_else(|| Failure::Usage {
            message: "--quiet-hours must be HH:MM-HH:MM, for example 22:00-07:00.".to_owned(),
            hint: None,
        })?;
        Ok(Some(set_device_policy::QuietHoursInput {
            start: start.to_owned(),
            end: end.to_owned(),
            time_zone: self.time_zone.clone().unwrap_or_default(),
        }))
    }
}

fn table(policy: &set_device_policy::SetDevicePolicySetDevicePolicy) -> Table {
    let mut table = Table::record();
    table.pair("Device", policy.device.name.as_str());
    table.pair("Memory share", format!("{}%", policy.memory_share_percent));
    table.pair("Disk share", format!("{}%", policy.disk_share_percent));
    let quiet = policy.quiet_hours.as_ref().map_or_else(
        || "-".to_owned(),
        |hours| format!("{}-{} {}", hours.start, hours.end, hours.time_zone),
    );
    table.pair("Quiet hours", quiet);
    table.pair("Mains only", policy.mains_only.to_string());
    table.pair("Idle only", policy.idle_only.to_string());
    table.pair("Takes model work", policy.takes_model_work.to_string());
    table
}
