//! `anima link devices`: the devices the link permits (LNK-004).

use super::documents::{LinkDevices, link_devices};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct DevicesArgs;

impl DevicesArgs {
    /// Prints the devices as a table or, with `--json`, as the K1 `LinkDevice` list.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (`UNAVAILABLE` when the hub has no link).
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let answer = hub.run::<LinkDevices>(link_devices::Variables).await?;
        invocation.output.data(&answer.link_devices, table)
    }
}

fn table(devices: &Vec<link_devices::LinkDevicesLinkDevices>) -> Table {
    let mut table = Table::new(vec![
        "NAME",
        "FINGERPRINT",
        "ADDRESS",
        "PATH",
        "EXPIRES",
        "STREAMS",
        "KEY",
    ]);
    for device in devices {
        let path = match &device.relay {
            Some(relay) => format!("{} ({relay})", enum_name(&device.path)),
            None => enum_name(&device.path),
        };
        table.row(vec![
            device.name.clone().into(),
            device.fingerprint.clone().into(),
            device.address.clone().into(),
            path.into(),
            device.expires_at.clone().unwrap_or_default().into(),
            device.streams.to_string().into(),
            device.key.clone().into(),
        ]);
    }
    table
}
