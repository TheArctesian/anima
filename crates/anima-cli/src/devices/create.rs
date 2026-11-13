//! `anima device create`: a device at a site, with its access settings as flags.

use serde_json::json;

use super::documents::{DeviceCreate, device_create};
use super::list::site_id;
use super::saved::Saved;
use super::settings::{Current, SettingsFlags, typed};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// The device's name.
    name: String,
    /// The site, by name or ID.
    #[arg(long)]
    site: String,
    /// The address or DNS name the hub reaches the device at.
    #[arg(long)]
    host: String,
    /// Do not add the default ICMP check.
    #[arg(long)]
    no_default_check: bool,
    #[command(flatten)]
    settings: SettingsFlags,
}

impl CreateArgs {
    /// Prints the new device or, with `--json`, the K1 `Device` fields selected.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown site; [`Failure::Usage`] for incomplete settings;
    /// the hub's refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let mut input = self.settings.fields(&Current::default())?;
        input.insert("siteId".to_owned(), json!(site_id(&hub, &self.site).await?));
        input.insert("name".to_owned(), json!(self.name));
        input.insert("host".to_owned(), json!(self.host));
        if self.no_default_check {
            input.insert("addDefaultCheck".to_owned(), json!(false));
        }
        let variables = device_create::Variables {
            input: typed(input)?,
        };
        let device = hub.run::<DeviceCreate>(variables).await?.create_device;
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
