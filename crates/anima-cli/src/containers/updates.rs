//! `anima containers updates <device>`: whether a newer image exists for each container
//! (HOST-021). The hub compares the digest on the device with the digest of the same tag at
//! its public registry. Anima updates nothing by itself (ETH-004).

use super::documents::{ContainerUpdateList, container_update_list};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table, Tone};

type Update = container_update_list::ContainerUpdateListContainerImageUpdates;

#[derive(Debug, clap::Args)]
pub struct UpdatesArgs {
    /// The device's ID or its exact name.
    device: String,
}

/// The answer for each container of the device with this ID or exact name.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device; the hub's refusal.
pub async fn fetch(hub: &Hub, device: &str) -> Result<Vec<Update>, Failure> {
    let device_id = resolve(hub, device).await?;
    let variables = container_update_list::Variables { device_id };
    Ok(hub
        .run::<ContainerUpdateList>(variables)
        .await?
        .container_image_updates)
}

impl UpdatesArgs {
    /// Prints a row for each container or, with `--json`, the K1 `ContainerImageUpdate` list.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal when the device has no
    /// SSH credential, has no Docker or does not answer.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let found = fetch(&invocation.hub()?, &self.device).await?;
        invocation.output.data(&found, table)
    }
}

/// `NEWER_IMAGE` as `newer image`.
fn words<T: serde::Serialize>(name: &T) -> String {
    enum_name(name).to_lowercase().replace('_', " ")
}

fn table(found: &Vec<Update>) -> Table {
    let mut table = Table::new(vec!["CONTAINER", "IMAGE", "STATE", "REASON"]);
    for row in found {
        let tone = match row.state {
            container_update_list::ContainerImageUpdateState::NEWER_IMAGE => Tone::Warning,
            container_update_list::ContainerImageUpdateState::UP_TO_DATE => Tone::Good,
            _ => Tone::Muted,
        };
        table.row(vec![
            row.container.as_str().into(),
            row.image.as_str().into(),
            Cell::toned(&words(&row.state), tone),
            row.reason
                .as_ref()
                .map_or_else(|| "-".to_owned(), words)
                .into(),
        ]);
    }
    table
}
