//! `anima device uptime`: the share of time the device was up over a window (CHK-011).

use super::documents::{DeviceUptime, device_uptime};
use super::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

#[derive(Debug, clap::Args)]
pub struct UptimeArgs {
    /// The device's ID or its exact name.
    device: String,
    /// How far back.
    #[arg(long, value_enum, default_value_t = WindowArg::Day)]
    window: WindowArg,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum WindowArg {
    Day,
    Week,
    Month,
    Year,
}

impl UptimeArgs {
    /// Prints the uptime as a percentage, or `-` when there is no history yet.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let variables = device_uptime::Variables {
            id: id.clone(),
            window: self.window.k1(),
        };
        let device = hub
            .run::<DeviceUptime>(variables)
            .await?
            .device
            .ok_or_else(|| Failure::NotFound {
                message: format!("No device has the ID {id}."),
                hint: Some("anima device list".to_owned()),
            })?;
        invocation.output.data(&device, |device| {
            let mut table = Table::record();
            table.pair("Device", device.name.as_str());
            table.pair(
                "Uptime",
                device
                    .uptime
                    .map_or_else(|| "-".to_owned(), |ratio| format!("{:.2} %", ratio * 100.0)),
            );
            table
        })
    }
}

impl WindowArg {
    const fn k1(self) -> device_uptime::UptimeWindow {
        match self {
            Self::Day => device_uptime::UptimeWindow::DAY,
            Self::Week => device_uptime::UptimeWindow::WEEK,
            Self::Month => device_uptime::UptimeWindow::MONTH,
            Self::Year => device_uptime::UptimeWindow::YEAR,
        }
    }
}
