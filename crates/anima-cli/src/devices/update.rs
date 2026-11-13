//! `anima device update`: changes the fields that have a flag and leaves the rest. A flag of a
//! settings group is laid over the group's current values, which are read first.

use serde_json::json;

use super::documents::{DeviceCurrent, DeviceUpdate, device_current, device_update};
use super::get::resolve;
use super::list::site_id;
use super::saved::Saved;
use super::settings::{Current, SettingsFlags, typed};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct UpdateArgs {
    /// The device's ID or its exact name.
    device: String,
    /// Move the device to this site, by name or ID.
    #[arg(long)]
    site: Option<String>,
    /// The new name.
    #[arg(long)]
    name: Option<String>,
    /// The new address or DNS name.
    #[arg(long)]
    host: Option<String>,
    /// Put the device in or out of manual maintenance.
    #[arg(long)]
    maintenance: Option<bool>,
    /// Remove every tag (`--tag` replaces them with the ones given).
    #[arg(long, conflicts_with = "tags")]
    clear_tags: bool,
    #[command(flatten)]
    settings: SettingsFlags,
}

impl UpdateArgs {
    /// Prints the device after the change.
    ///
    /// # Errors
    /// [`Failure::Usage`] when no flag changes anything or a settings group is incomplete;
    /// [`Failure::NotFound`] for an unknown device or site; the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let id = resolve(&hub, &self.device).await?;
        let current = if self.settings.touches_groups() {
            let answer = hub
                .run::<DeviceCurrent>(device_current::Variables { id: id.clone() })
                .await?;
            let device = answer.device.ok_or_else(|| Failure::NotFound {
                message: format!("No device has the ID {id}."),
                hint: Some("anima device list".to_owned()),
            })?;
            Current::of(&device)
        } else {
            Current::default()
        };
        let mut input = self.settings.fields(&current)?;
        if self.clear_tags {
            input.insert("tags".to_owned(), json!([]));
        }
        if let Some(site) = &self.site {
            input.insert("siteId".to_owned(), json!(site_id(&hub, site).await?));
        }
        for (key, value) in [("name", &self.name), ("host", &self.host)] {
            if let Some(value) = value {
                input.insert(key.to_owned(), json!(value));
            }
        }
        if let Some(maintenance) = self.maintenance {
            input.insert("maintenance".to_owned(), json!(maintenance));
        }
        if input.is_empty() {
            return Err(Failure::Usage {
                message: "Nothing to change: give at least one flag.".to_owned(),
                hint: Some("anima device update --help".to_owned()),
            });
        }
        let variables = device_update::Variables {
            id,
            input: typed(input)?,
        };
        let device = hub.run::<DeviceUpdate>(variables).await?.update_device;
        invocation.output.data(&device, |device| {
            Saved {
                id: &device.id,
                name: &device.name,
                host: &device.host,
                site: &device.site.name,
                status: &enum_name(&device.status),
            }
            .table()
        })
    }
}
