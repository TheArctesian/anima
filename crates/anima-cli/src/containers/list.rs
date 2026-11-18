//! `anima containers list <device>`: the containers of a device, running or not (HOST-001).

use super::documents::{ContainerList, container_list};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

/// The digits of a container identifier the table shows, as `docker ps` does.
const SHORT_ID: usize = 12;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The device's ID or its exact name.
    device: String,
}

impl ListArgs {
    /// Prints a row for each container or, with `--json`, the K1 `Container` list.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal when the device has no
    /// SSH credential, has no Docker or does not answer.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let found = fetch(&hub, &self.device).await?;
        invocation.output.data(&found, table)
    }
}

/// The containers of the device with this ID or exact name.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device; the hub's refusal.
pub async fn fetch(
    hub: &Hub,
    device: &str,
) -> Result<Vec<container_list::ContainerListContainers>, Failure> {
    let device_id = resolve(hub, device).await?;
    let variables = container_list::Variables { device_id };
    Ok(hub.run::<ContainerList>(variables).await?.containers)
}

/// The identifier the table shows and `containers logs` accepts.
#[must_use]
pub fn short_id(id: &str) -> &str {
    id.get(..SHORT_ID).unwrap_or(id)
}

fn table(found: &Vec<container_list::ContainerListContainers>) -> Table {
    let mut table = Table::new(vec![
        "ID", "NAME", "STATE", "STATUS", "IMAGE", "PORTS", "PROJECT",
    ]);
    for row in found {
        table.row(vec![
            short_id(&row.id).into(),
            row.name.as_str().into(),
            Cell::status(&enum_name(&row.state)),
            row.status.as_str().into(),
            row.image.as_str().into(),
            Cell::new(&row.ports.join(", ")),
            row.compose_project.as_deref().unwrap_or("-").into(),
        ]);
    }
    table
}
