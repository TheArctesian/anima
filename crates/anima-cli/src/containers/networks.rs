//! `anima containers networks <device>`: the networks of Docker on a device, each with the
//! containers on it (HOST-020).

use super::documents::{ContainerNetworkList, container_network_list};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

type Network = container_network_list::ContainerNetworkListContainerNetworks;

/// The digits of a network identifier the table shows, as `docker network ls` does.
const SHORT_ID: usize = 12;

#[derive(Debug, clap::Args)]
pub struct NetworksArgs {
    /// The device's ID or its exact name.
    device: String,
}

/// The networks of the device with this ID or exact name.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device; the hub's refusal.
pub async fn fetch(hub: &Hub, device: &str) -> Result<Vec<Network>, Failure> {
    let device_id = resolve(hub, device).await?;
    let variables = container_network_list::Variables { device_id };
    Ok(hub
        .run::<ContainerNetworkList>(variables)
        .await?
        .container_networks)
}

impl NetworksArgs {
    /// Prints a row for each network or, with `--json`, the K1 `ContainerNetwork` list.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal when the device has no
    /// SSH credential, has no Docker or does not answer.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let found = fetch(&invocation.hub()?, &self.device).await?;
        invocation.output.data(&found, table)
    }
}

fn table(found: &Vec<Network>) -> Table {
    let mut table = Table::new(vec![
        "ID",
        "NAME",
        "DRIVER",
        "SCOPE",
        "INTERNAL",
        "CONTAINERS",
    ]);
    for row in found {
        table.row(vec![
            row.id.get(..SHORT_ID).unwrap_or(&row.id).into(),
            row.name.as_str().into(),
            row.driver.as_str().into(),
            row.scope.as_str().into(),
            if row.internal { "yes" } else { "no" }.into(),
            row.containers.join(", ").into(),
        ]);
    }
    table
}
