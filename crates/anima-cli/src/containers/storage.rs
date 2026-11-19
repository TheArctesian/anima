//! `anima containers images <device>` and `anima containers volumes <device>`: what Docker
//! keeps on a device, with the size of each and the space that a clean-up can free
//! (HOST-020). Read-only: Anima removes nothing here.

use super::documents::{ContainerStorageRead, container_storage_read};
use crate::devices::get::resolve;
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::Table;

pub type Storage = container_storage_read::ContainerStorageReadContainerStorage;

/// The digits of an image identifier the table shows, as `docker image ls` does.
const SHORT_ID: usize = 12;

#[derive(Debug, clap::Args)]
pub struct StorageArgs {
    /// The device's ID or its exact name.
    device: String,
}

/// The images and the volumes of the device with this ID or exact name.
///
/// # Errors
/// [`Failure::NotFound`] for an unknown device; the hub's refusal.
pub async fn fetch(hub: &Hub, device: &str) -> Result<Storage, Failure> {
    let device_id = resolve(hub, device).await?;
    let variables = container_storage_read::Variables { device_id };
    Ok(hub
        .run::<ContainerStorageRead>(variables)
        .await?
        .container_storage)
}

impl StorageArgs {
    /// Prints a row for each image or, with `--json`, the K1 `ContainerImage` list.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal when the device has no
    /// SSH credential, has no Docker or does not answer.
    pub async fn images(self, invocation: &Invocation) -> Result<(), Failure> {
        let found = fetch(&invocation.hub()?, &self.device).await?;
        invocation.output.data(&found.images, images_table)?;
        reclaimable(invocation, &found, "Images");
        Ok(())
    }

    /// Prints a row for each volume or, with `--json`, the K1 `ContainerVolume` list.
    ///
    /// # Errors
    /// As [`Self::images`].
    pub async fn volumes(self, invocation: &Invocation) -> Result<(), Failure> {
        let found = fetch(&invocation.hub()?, &self.device).await?;
        invocation.output.data(&found.volumes, volumes_table)?;
        reclaimable(invocation, &found, "Local Volumes");
        Ok(())
    }
}

/// Says, apart from the rows, how much space a clean-up of this type can free.
fn reclaimable(invocation: &Invocation, found: &Storage, kind: &str) {
    if invocation.output.json() {
        return;
    }
    if let Some(usage) = found.usage.iter().find(|usage| usage.type_ == kind) {
        invocation.output.note(&format!(
            "{kind}: {} in use by Docker, {} can be freed. Anima removes nothing here.",
            usage.size, usage.reclaimable
        ));
    }
}

/// The names of the containers, with how many more Docker counts when the hub found not
/// all of them; the number alone when it found none.
fn users(count: Option<i64>, names: &[String]) -> String {
    let found = i64::try_from(names.len()).unwrap_or(i64::MAX);
    match count {
        _ if names.is_empty() => count.map_or_else(|| "-".to_owned(), |count| count.to_string()),
        Some(count) if count > found => format!("{} (+{})", names.join(", "), count - found),
        _ => names.join(", "),
    }
}

fn images_table(
    found: &Vec<container_storage_read::ContainerStorageReadContainerStorageImages>,
) -> Table {
    let mut table = Table::new(vec![
        "ID",
        "REPOSITORY",
        "TAG",
        "SIZE",
        "UNIQUE",
        "CONTAINERS",
    ]);
    for row in found {
        let digits = row.id.trim_start_matches("sha256:");
        table.row(vec![
            digits.get(..SHORT_ID).unwrap_or(digits).into(),
            row.repository.as_deref().unwrap_or("<none>").into(),
            row.tag.as_deref().unwrap_or("<none>").into(),
            row.size.as_str().into(),
            row.unique_size.as_str().into(),
            users(row.container_count, &row.containers).into(),
        ]);
    }
    table
}

fn volumes_table(
    found: &Vec<container_storage_read::ContainerStorageReadContainerStorageVolumes>,
) -> Table {
    let mut table = Table::new(vec!["NAME", "DRIVER", "SIZE", "CONTAINERS"]);
    for row in found {
        table.row(vec![
            row.name.as_str().into(),
            row.driver.as_str().into(),
            row.size.as_str().into(),
            users(row.container_count, &row.containers).into(),
        ]);
    }
    table
}
